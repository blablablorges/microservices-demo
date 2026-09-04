use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

pub mod hipstershop {
    tonic::include_proto!("hipstershop");
}

mod core;

use core::CartStore;
use hipstershop::cart_service_server::{CartService, CartServiceServer};
use hipstershop::{AddItemRequest, Cart, Empty, EmptyCartRequest, GetCartRequest};

// ---------------------------------------------------------------------------
// Redis adapter — one multiplexed connection for the process lifetime, as
// StackExchange.Redis does in the upstream cart; re-dialed after an error.
// The same source on native and wasip2.
// ---------------------------------------------------------------------------

struct RedisStore {
    client: redis::Client,
    conn: Mutex<Option<MultiplexedConnection>>,
}

impl RedisStore {
    async fn conn(&self) -> Result<MultiplexedConnection, String> {
        let cached = self.conn.lock().unwrap().clone();
        if let Some(c) = cached {
            return Ok(c);
        }
        let c = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| e.to_string())?;
        *self.conn.lock().unwrap() = Some(c.clone());
        Ok(c)
    }

    fn failed(&self, e: redis::RedisError) -> String {
        if e.is_connection_dropped() || e.is_io_error() {
            *self.conn.lock().unwrap() = None;
        }
        e.to_string()
    }
}

#[tonic::async_trait]
impl CartStore for RedisStore {
    async fn load(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let mut conn = self.conn().await?;
        conn.get(key).await.map_err(|e| self.failed(e))
    }

    async fn save(&self, key: &str, data: Vec<u8>) -> Result<(), String> {
        let mut conn = self.conn().await?;
        conn.set::<_, _, ()>(key, data).await.map_err(|e| self.failed(e))
    }
}

// ---------------------------------------------------------------------------
// gRPC service — delegates entirely to core logic
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct CartServiceImpl {
    store: Arc<RedisStore>,
}

#[tonic::async_trait]
impl CartService for CartServiceImpl {
    async fn add_item(
        &self,
        request: Request<AddItemRequest>,
    ) -> Result<Response<Empty>, Status> {
        core::add_item(self.store.as_ref(), request.into_inner())
            .await
            .map(Response::new)
    }

    async fn get_cart(
        &self,
        request: Request<GetCartRequest>,
    ) -> Result<Response<Cart>, Status> {
        core::get_cart(self.store.as_ref(), request.into_inner().user_id)
            .await
            .map(Response::new)
    }

    async fn empty_cart(
        &self,
        request: Request<EmptyCartRequest>,
    ) -> Result<Response<Empty>, Status> {
        core::empty_cart(self.store.as_ref(), request.into_inner().user_id)
            .await
            .map(Response::new)
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

// Resolved once at startup: tokio's resolver falls back to spawn_blocking for a
// hostname, and wasip2 has no threads (found in-cluster by the synthetic port).
fn redis_url(addr: &str) -> Result<String> {
    let hostport = addr.strip_prefix("redis://").unwrap_or(addr).trim_end_matches('/');
    let sock = std::net::ToSocketAddrs::to_socket_addrs(hostport)
        .ok()
        .and_then(|mut it| it.next())
        .with_context(|| format!("cannot resolve {}", addr))?;
    Ok(format!("redis://{}/", sock))
}

// WP-J1/H3: one source for both targets. current_thread on both — wasip2 has no
// threads, and the schedule has to be identical for the comparison to be fair.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "7070".to_string())
        .parse()
        .context("invalid PORT")?;
    let redis_addr =
        std::env::var("REDIS_ADDR").unwrap_or_else(|_| "redis-cart:6379".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .context("bind failed")?;

    let client = redis::Client::open(redis_url(&redis_addr)?)
        .context("failed to create Redis client")?;
    let service = CartServiceImpl {
        store: Arc::new(RedisStore { client, conn: Mutex::new(None) }),
    };

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<CartServiceServer<CartServiceImpl>>()
        .await;

    println!("CartService listening on {} (redis={})", addr, redis_addr);

    Server::builder()
        .add_service(health_service)
        .add_service(CartServiceServer::new(service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .context("gRPC server failed")?;

    Ok(())
}
