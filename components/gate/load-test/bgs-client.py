#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
#
# BGS WebSocket client for testing the login queue.
#
# Connects to the BGS server (default ws://localhost:8119) and sends a
# LogonRequest with allow_logon_queue_notifications=true. If queued,
# prints queue position updates until LogonQueueEnd is received.
#
# Usage:
#   python3 load-test/bgs-client.py --server ws://localhost:8119
#   python3 load-test/bgs-client.py --server ws://localhost:8119 --email test@example.com

import argparse
import json
import os
import struct
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "bgs_proto"))
import bgs_pb2  # noqa: E402

try:
    import websocket  # noqa: E402
except ImportError:
    print("install websocket-client: pip3 install --user websocket-client")
    sys.exit(1)

# Service hash constants (from tavern-bgs/service_hash.rs)
AUTHENTICATION_CLIENT_V1 = 0x71240E35
AUTHENTICATION_SERVER_V1 = 0x0DECFC01
CONNECTION_SERVICE = 0x65446991
ACCOUNT_SERVICE = 0x62DA0891
GET_ACCOUNT_STATE = 30
GET_GAME_ACCOUNT_STATE = 31

# BGS result code constants (from tavern-bgs/result_code.rs)
ERROR_OK = 0
ERROR_GAME_ACCOUNT_NO_TIME = 30
ERROR_GAME_ACCOUNT_SUSPENDED = 33
ERROR_GAME_ACCOUNT_BANNED = 52
ERROR_SESSION_DUPLICATE = 60
ERROR_SESSION_DISCONNECTED = 61
ERROR_ADMIN_KICK = 70
ERROR_SERVER_SHUTTING_DOWN = 92
ERROR_BATTLENET_ACCOUNT_BANNED = 96

ERROR_LABELS = {
    0: "ok",
    30: "no game time",
    33: "suspended",
    52: "banned",
    60: "session duplicate",
    61: "disconnected",
    70: "admin kick",
    92: "server shutdown",
    96: "bnet account banned",
}


def make_header(service_hash, method_id, token=0, is_response=False):
    """Build a Header protobuf message."""
    h = bgs_pb2.Header()
    h.service_id = 1
    h.service_hash = service_hash
    if method_id is not None:
        h.method_id = method_id
    h.token = token
    if is_response:
        h.is_response = True
    return h


def frame_message(header, body_bytes=None):
    """Serialise a frame: 2-byte BE header size + Header + optional body."""
    header_bytes = header.SerializeToString()
    frame = struct.pack(">H", len(header_bytes))
    frame += header_bytes
    if body_bytes:
        frame += body_bytes
    return frame


def parse_frame(data):
    """Parse a binary frame into (header, body_bytes_or_None)."""
    if len(data) < 2:
        return None, None
    hdr_len = struct.unpack(">H", data[:2])[0]
    if len(data) < 2 + hdr_len:
        return None, None
    header_bytes = data[2 : 2 + hdr_len]
    header = bgs_pb2.Header()
    header.ParseFromString(header_bytes)
    body_start = 2 + hdr_len
    body = data[body_start:] if body_start < len(data) else None
    return header, body


def run_logon(ws, email, ticket=None):
    """Logon + optionally VerifyWebCredentials + GenerateSSOToken.
    Returns SSO token bytes if available."""
    lr = bgs_pb2.LogonRequest()
    lr.program = "WoW"
    lr.platform = "Win"
    lr.locale = "enUS"
    lr.email = email
    lr.application_version = 31650
    lr.allow_logon_queue_notifications = True
    ws.send_binary(frame_message(make_header(AUTHENTICATION_SERVER_V1, 1), lr.SerializeToString()))

    hdr, body = recv_or_fail(ws)
    if hdr is None:
        print("Logon: no response")
        return None

    # Handle queue
    while hdr.method_id == 12:
        q = bgs_pb2.LogonQueueUpdateRequest()
        if body:
            q.ParseFromString(body)
        print(f"  Queue: position={q.position}")
        hdr, body = recv_or_fail(ws)

    if hdr.is_response:
        print("  Logon: NoData")
    else:
        print(f"  Logon: unexpected method {hdr.method_id}")
        return None

    if not ticket:
        return None

    # VerifyWebCredentials
    vw = bgs_pb2.VerifyWebCredentialsRequest()
    vw.web_credentials = ticket.encode()
    ws.send_binary(frame_message(make_header(AUTHENTICATION_SERVER_V1, 7), vw.SerializeToString()))
    hdr, _ = recv_or_fail(ws)
    if hdr.is_response and hdr.status == 0:
        print("  VWC: ok")
    else:
        print(f"  VWC: fail ({hdr.status})")
        return None

    # Read OnLogonComplete push
    hdr, body = recv_or_fail(ws)
    if hdr.method_id == 5:
        print("  OnLogonComplete: received")
        lr = bgs_pb2.LogonResult()
        if body:
            lr.ParseFromString(body)
        query_account_state(ws, lr.game_account_id)

    # GenerateSSOToken
    gs = bgs_pb2.GenerateSSOTokenRequest()
    ws.send_binary(frame_message(make_header(AUTHENTICATION_SERVER_V1, 5), gs.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response and hdr.status == 0 and body:
        resp = bgs_pb2.GenerateSSOTokenResponse()
        resp.ParseFromString(body)
        if resp.sso_id:
            print(f"  SSOT: {resp.sso_id.hex()[:16]}...")
            return resp.sso_id
    print("  SSOT: none")
    return None


def query_account_state(ws, game_account_ids):
    """Mirror the 1.13.2 login gate: pull the account state, then the
    first offered game account's state.

    The real client never sends SelectGameAccount (auth method 6 is
    deprecated and undispatched; the realm join carries the selection).
    The gate instead reads AccountService GetAccountState (method 30,
    the game-account list) and GetGameAccountState (method 31, the
    selected account's licenses/game-time/status), then auto-selects
    when exactly one account is offered. This helper replays those two
    calls so the test observes the same server data the client gate
    consumes."""
    if len(game_account_ids) == 0:
        return

    # GetAccountState: the account-level state + game-account list.
    names = {}
    try:
        ws.send_binary(frame_message(make_header(ACCOUNT_SERVICE, GET_ACCOUNT_STATE), None))
        hdr, body = recv_or_fail(ws)
        if hdr.is_response and hdr.status == 0 and body:
            resp = bgs_pb2.GetAccountStateResponse()
            resp.ParseFromString(body)
            ids = [h.id for gl in resp.state.game_accounts for h in gl.handle]
            for ga_id, gli in zip(ids, resp.state.game_level_info):
                if gli.name:
                    names[ga_id] = gli.name
    except Exception:
        pass  # continue with raw ids if GetAccountState fails

    first = game_account_ids[0]
    picked = names.get(first.low, str(first.low))
    print(
        f"  AccountState: {len(game_account_ids)} game account(s), ",
        f"selected {picked} (id {first.low})",
    )

    # GetGameAccountState for the selected account (the gate's data source).
    try:
        req = bgs_pb2.GetGameAccountStateRequest()
        req.game_account_id.CopyFrom(first)
        ws.send_binary(
            frame_message(
                make_header(ACCOUNT_SERVICE, GET_GAME_ACCOUNT_STATE),
                req.SerializeToString(),
            )
        )
        hdr, body = recv_or_fail(ws)
        if hdr.is_response and hdr.status == 0 and body:
            resp = bgs_pb2.GetGameAccountStateResponse()
            resp.ParseFromString(body)
            gli = resp.state.game_level_info
            print(
                "  GameAccountState: name=",
                gli.name if gli.name else "?",
                " licenses=",
                [lic.id for lic in gli.licenses],
            )
        else:
            print(f"  GameAccountState: fail ({hdr.status})")
    except Exception:
        print("  GameAccountState: query failed")


def run_restore(ws, sso_id_hex):
    """Restore session with a previously obtained SSO token."""
    sid = bgs_pb2.SessionId()
    sid.instance_id = sso_id_hex
    ws.send_binary(frame_message(make_header(0x7E9859A3, 2), sid.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response and hdr.status == 0:
        print("  RestoreSession: ok")
        hdr, body = recv_or_fail(ws)
        if hdr.method_id == 5:
            print("  OnLogonComplete: received")
            lr = bgs_pb2.LogonResult()
            if body:
                lr.ParseFromString(body)
            query_account_state(ws, lr.game_account_id)
    else:
        print(f"  RestoreSession: fail ({hdr.status})")


def run_character_list(ws):
    """After OnLogonComplete, send GameUtilities commands to reach character list."""
    gu_hash = 0x3FC1274D  # GAME_UTILITIES_V1

    # 1. RealmListTicketRequest
    req = bgs_pb2.ClientRequest()
    req.attribute.extend(
        [
            bgs_pb2.Attribute(name="Command_RealmListTicketRequest_v1", value=bgs_pb2.Variant()),
        ]
    )
    ws.send_binary(frame_message(make_header(gu_hash, 1), req.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response:
        print("  RealmListTicket: ok")

    # 2. RealmListRequest
    req2 = bgs_pb2.ClientRequest()
    req2.attribute.extend(
        [
            bgs_pb2.Attribute(name="Command_RealmListRequest_v1", value=bgs_pb2.Variant()),
        ]
    )
    ws.send_binary(frame_message(make_header(gu_hash, 1), req2.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response:
        print("  RealmList: ok")

    # 3. LastCharPlayed
    req3 = bgs_pb2.ClientRequest()
    req3.attribute.extend(
        [
            bgs_pb2.Attribute(name="Command_LastCharPlayedRequest_v1", value=bgs_pb2.Variant()),
        ]
    )
    ws.send_binary(frame_message(make_header(gu_hash, 1), req3.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response:
        print("  LastCharPlayed: ok")

    # 4. CharacterListRequest
    req4 = bgs_pb2.ClientRequest()
    req4.attribute.extend(
        [
            bgs_pb2.Attribute(name="Command_CharacterListRequest_v1", value=bgs_pb2.Variant()),
        ]
    )
    ws.send_binary(frame_message(make_header(gu_hash, 1), req4.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response:
        print("  CharacterList: ok")

    # 5. RealmJoinRequest — the pre-realm-join handoff. The server must
    # answer with ServerAddresses (zlib JSON), JoinSecret (32 bytes),
    # RealmJoinTicket and BnetSessionKey (realmlist-flow.md §RealmJoin).
    req5 = bgs_pb2.ClientRequest()
    ra = bgs_pb2.Attribute(name="Param_RealmAddress")
    ra.value.uint_value = 0x01010100
    req5.attribute.extend(
        [
            bgs_pb2.Attribute(name="Command_RealmJoinRequest_v1", value=bgs_pb2.Variant()),
            ra,
        ]
    )
    ws.send_binary(frame_message(make_header(gu_hash, 1), req5.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr.is_response and hdr.status == 0:
        resp = bgs_pb2.ClientResponse()
        resp.ParseFromString(body)
        by_name = {a.name: a.value.blob_value for a in resp.attribute}
        assert "Param_ServerAddresses" in by_name, "missing Param_ServerAddresses"
        assert "Param_JoinSecret" in by_name, "missing Param_JoinSecret"
        assert "Param_RealmJoinTicket" in by_name, "missing Param_RealmJoinTicket"
        assert "Param_BnetSessionKey" in by_name, "missing Param_BnetSessionKey"
        assert len(by_name["Param_JoinSecret"]) == 32, "JoinSecret must be 32 bytes"

        # ServerAddresses: 4-byte LE uncompressed length + zlib JSON.
        import zlib

        blob = by_name["Param_ServerAddresses"]
        payload = zlib.decompress(blob[4:])
        # TC convention: the payload carries a trailing NUL (prefix = len+1).
        text = payload.decode("utf-8").rstrip("\x00")
        addrs = json.loads(text[len("JSONRealmListServerIPAddresses:"):])
        family = addrs["families"][0]
        assert family["family"] in (1, 2)
        assert family["addresses"][0]["ip"]
        assert family["addresses"][0]["port"] > 0
        print(f"  RealmJoin: ok ({family['addresses'][0]['ip']}:{family['addresses'][0]['port']})")
    else:
        print(f"  RealmJoin: fail (status {hdr.status})")
        sys.exit(1)

class DisconnectError(Exception):
    """Raised when the server sends a ForceDisconnect frame."""

    def __init__(self, error_code, reason):
        self.error_code = error_code
        self.reason = reason
        label = ERROR_LABELS.get(error_code, f"code {error_code}")
        msg = f"ForceDisconnect: {label}"
        if reason:
            msg += f" ({reason})"
        super().__init__(msg)


def recv_or_fail(ws):
    """Receive a frame; raise DisconnectError if it is a ForceDisconnect."""
    data = ws.recv()
    hdr, body = parse_frame(data)
    if hdr is not None and hdr.service_hash == CONNECTION_SERVICE and hdr.method_id == 4:
        code = 0
        reason = ""
        if body:
            dn = bgs_pb2.DisconnectNotification()
            dn.ParseFromString(body)
            code = dn.error_code
            if dn.HasField("reason"):
                reason = dn.reason
        raise DisconnectError(code, reason)
    return hdr, body


def read_disconnect(ws):
    """Try to read a server-initiated disconnect frame and show the reason."""
    try:
        ws.settimeout(1.0)
        recv_or_fail(ws)
        return True
    except DisconnectError as e:
        print(f"  {e}")
        return True
    except Exception:
        pass
    finally:
        ws.settimeout(30)
    return False


def do_disconnect(ws):
    """Send a proper disconnect/logout frame."""
    try:
        ws.send_binary(
            frame_message(make_header(0x65446991, 7), None)
        )  # ConnectionService::RequestDisconnect
    except Exception:
        pass


def run(server, email, password, flow, sso_id="", ticket=None):
    try:
        _run(server, email, password, flow, sso_id, ticket)
    except DisconnectError as e:
        print(f"  {e}")


def _run(server, email, password, flow, sso_id="", ticket=None):
    ws = websocket.create_connection(server, subprotocols=["v1.rpc.battle.net"], timeout=30)
    print(f"Connected to {server}")

    # Connect
    cr = bgs_pb2.ConnectRequest()
    cr.use_bindless_rpc = True
    ws.send_binary(frame_message(make_header(CONNECTION_SERVICE, 1), cr.SerializeToString()))
    hdr, body = recv_or_fail(ws)
    if hdr and hdr.is_response:
        cr_resp = bgs_pb2.ConnectResponse()
        if body:
            cr_resp.ParseFromString(body)
        print(
            f"ConnectResponse: server_id={cr_resp.server_id.label if cr_resp.HasField('server_id') else '?'}"
        )

    if flow in ("logon", "full", "character"):  # noqa: E501
        # The ticket is single-use: the server marks it used on
        # VerifyWebCredentials. The caller supplies a fresh one (e.g. from
        # dev/ticket.sh); BTEST-1 remains the backward-compatible default.
        ticket = ticket if ticket else ("BTEST-1" if flow != "logon" else None)
        sso = run_logon(ws, email, ticket)

        if flow in ("full", "character") and sso:
            ws.close()
            ws2 = websocket.create_connection(server, subprotocols=["v1.rpc.battle.net"], timeout=30)
            ws2.send_binary(
                frame_message(
                    make_header(CONNECTION_SERVICE, 1),
                    bgs_pb2.ConnectRequest(use_bindless_rpc=True).SerializeToString(),
                )
            )
            parse_frame(ws2.recv())
            run_restore(ws2, sso.hex())
            if flow == "character":
                run_character_list(ws2)
            do_disconnect(ws2)
            ws2.close()
    elif flow == "restore":
        run_restore(ws, sso_id)

    do_disconnect(ws)
    ws.close()
    print("Done")


def main():
    p = argparse.ArgumentParser(
        prog="bgs-client",
        description="BGS WebSocket login client — test all auth flows",
    )
    p.add_argument("--server", "-s", default="ws://localhost:8119")
    p.add_argument("--email", "-e", default="test@example.com")
    p.add_argument("--password", "-p", default="irrelevant")
    p.add_argument(
        "--flow",
        "-f",
        default="logon",
        choices=["logon", "full", "restore", "character"],
        help="logon=Logon only, full=auth+SSOT+Restore, restore=RestoreSession, character=full flow",
    )
    p.add_argument("--sso-id", default="", help="SSO token hex for --flow restore")
    p.add_argument(
        "--ticket",
        default=None,
        help="Service ticket for VerifyWebCredentials (default BTEST-1)",
    )
    args = p.parse_args()  # noqa: E501
    run(args.server, args.email, args.password, args.flow, args.sso_id, args.ticket)


if __name__ == "__main__":
    main()
