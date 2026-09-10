// SPDX-License-Identifier: AGPL-3.0-only

//! Observable gauges reporting the sqlx connection pool state.

use opentelemetry::KeyValue;
use sqlx::PgPool;

/// Register a `db.client.connections.usage` observable gauge that reports pool
/// connections split by `state` (`idle`, `used`). The SDK polls the callback on
/// its periodic export cadence.
pub fn register_pool_gauge(pool: PgPool) {
    let meter = opentelemetry::global::meter("tavern");
    let pool = pool;

    meter
        .u64_observable_gauge("db.client.connections.usage")
        .with_unit("{connection}")
        .with_description("Active database pool connections by state")
        .with_callback(move |observer| {
            let idle = pool.num_idle() as u64;
            let size = pool.size() as u64;
            observer.observe(idle, &[KeyValue::new("state", "idle")]);
            observer.observe(size.saturating_sub(idle), &[KeyValue::new("state", "used")]);
        })
        .build();
}
