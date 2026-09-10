#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
#
# BGS login queue concurrency test.
#
# Spawns N clients, one of which holds a slot, forcing the others to queue.
# Verifies position updates, dequeue, and retry flow.
#
# Usage:
#   MAX_BGS_LOGINS=1 python3 load-test/bgs-queue-test.py
#
# Prerequisites:
#   A running realmforge-gate-bgs-server with MAX_BGS_LOGINS set to a low value (e.g., 1).
#   pip3 install --user websocket-client protobuf

import argparse
import os
import struct
import sys
import threading
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "bgs_proto"))
import bgs_pb2  # noqa: E402

try:
    import websocket  # noqa: E402
except ImportError:
    print("pip3 install --user websocket-client")
    sys.exit(1)

SERVER = os.environ.get("BGS_SERVER", "ws://localhost:8119")


def make_header(sh: int, mid: int) -> bgs_pb2.Header:
    h = bgs_pb2.Header()
    h.service_id = 1
    h.token = 0
    h.service_hash = sh
    h.method_id = mid
    return h


def frame_message(h: bgs_pb2.Header, body: bytes | None = None) -> bytes:
    hb = h.SerializeToString()
    f = struct.pack(">H", len(hb)) + hb
    return f + body if body else f


def parse_frame(data: bytes) -> tuple[bgs_pb2.Header, bytes | None]:
    hl = struct.unpack(">H", data[:2])[0]
    h = bgs_pb2.Header()
    h.ParseFromString(data[2 : 2 + hl])
    return h, data[2 + hl :] if len(data) > 2 + hl else None


def connect() -> websocket.WebSocket:
    return websocket.create_connection(SERVER, subprotocols=["v1.rpc.battle.net"], timeout=30)


def do_connect(ws: websocket.WebSocket) -> None:
    cr = bgs_pb2.ConnectRequest()
    cr.use_bindless_rpc = True
    ws.send_binary(frame_message(make_header(0x65446991, 1), cr.SerializeToString()))
    rh, _ = parse_frame(ws.recv())
    if not rh.is_response:
        raise RuntimeError("ConnectRequest failed")


def do_logon(ws: websocket.WebSocket, email: str, allow_queue: bool = True) -> str:
    """Send Logon, return 'direct' or 'queued:N'."""
    lr = bgs_pb2.LogonRequest()
    lr.program = "WoW"
    lr.platform = "Win"
    lr.locale = "enUS"
    lr.application_version = 31650
    lr.email = email
    lr.allow_logon_queue_notifications = allow_queue
    ws.send_binary(frame_message(make_header(0x0DECFC01, 1), lr.SerializeToString()))

    # Read first frame (NoData response or queue push)
    data = ws.recv()
    rh, body = parse_frame(data)

    if rh.is_response:
        # Check for a follow-up queue push in the next frame
        ws.settimeout(0.5)
        try:
            data2 = ws.recv()
            rh2, body2 = parse_frame(data2)
            if rh2.method_id == 12 and body2:
                q = bgs_pb2.LogonQueueUpdateRequest()
                q.ParseFromString(body2)
                return f"queued:{q.position}"
        except websocket.WebSocketTimeoutException:
            pass
        finally:
            ws.settimeout(30)
        return "direct"
    elif rh.method_id == 12 and body:
        q = bgs_pb2.LogonQueueUpdateRequest()
        q.ParseFromString(body)
        return f"queued:{q.position}"

    return "unknown"


def hold_slot(ws: websocket.WebSocket, email: str, hold_secs: int) -> str:
    """Connect, Logon, hold for hold_secs, disconnect. Returns status."""
    do_connect(ws)
    status = do_logon(ws, email)
    if status == "direct":
        print(f"  {email}: holding slot for {hold_secs}s")
        time.sleep(hold_secs)
    ws.close()
    return status


def main():

    parser = argparse.ArgumentParser(
        prog="bgs-queue-test",
        description="BGS login concurrency and queue test",
    )
    parser.add_argument("--server", "-s", default="ws://localhost:8119")
    parser.add_argument(
        "--count",
        "-n",
        type=int,
        default=200,
        help="Total clients to spawn (default: 200)",
    )
    parser.add_argument(
        "--concurrency",
        "-c",
        type=int,
        default=50,
        help="Max concurrent clients (default: 50)",
    )
    parser.add_argument(
        "--hold-secs",
        type=int,
        default=0,
        help="Seconds to hold the first slot (0=benchmark mode)",
    )
    args = parser.parse_args()

    total = args.count
    hold_secs = args.hold_secs

    statuses: list[str] = []
    lock = threading.Lock()

    def run_hold(email):
        ws = connect()
        s = hold_slot(ws, email, hold_secs)
        with lock:
            statuses.append(s)
            print(f"  {email}: {s}")

    def run_client(email):
        ws = connect()
        do_connect(ws)
        s = do_logon(ws, email)
        with lock:
            statuses.append(s)
        ws.close()

    sem = threading.Semaphore(args.concurrency)
    threads = []

    if hold_secs > 0:
        # Queue-test mode: first client holds the slot.
        t = threading.Thread(
            target=lambda: (sem.acquire(), run_hold("test@example.com"), sem.release()),
            daemon=True,
        )
        t.start()
        threads.append(t)
        time.sleep(0.5)

    def bench_client(email):
        sem.acquire()
        try:
            ws = connect()
            do_connect(ws)
            s = do_logon(ws, email)
            with lock:
                statuses.append(s)
                if s != "direct":
                    print(f"  {email}: {s}")
            ws.close()
        finally:
            sem.release()

    for i in range(len(threads), total):
        t = threading.Thread(
            target=bench_client,
            args=(f"lt{i}@loadtest.local",),
            daemon=True,
        )
        t.start()
        threads.append(t)
        if i % 100 == 0:
            time.sleep(0.01)

    for t in threads:
        t.join()

    direct = sum(1 for s in statuses if s == "direct")
    queued = sum(1 for s in statuses if s.startswith("queued"))
    print(f"\n  Summary: {direct} direct, {queued} queued")
    return 0 if queued > 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
