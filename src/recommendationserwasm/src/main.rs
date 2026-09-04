use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

pub mod hipstershop {
    tonic::include_proto!("hipstershop");
}

mod core;

use core::CatalogClient;
use hipstershop::product_catalog_service_client::ProductCatalogServiceClient;
use hipstershop::recommendation_service_server::{RecommendationService, RecommendationServiceServer};
use hipstershop::{Empty, ListRecommendationsRequest, ListRecommendationsResponse};

// One lazy tonic channel for the process lifetime, as the Python service holds
// one grpc channel. See src/lib/tonic/HYRBID-PATCH.md for what it took to make
// `channel` build on wasm32-wasip2.
struct TonicCatalogClient {
    channel: Channel,
}

#[tonic::async_trait]
impl CatalogClient for TonicCatalogClient {
    async fn list_product_ids(&self) -> Result<Vec<String>, String> {
        let resp = ProductCatalogServiceClient::new(self.channel.clone())
            .list_products(Empty {})
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.into_inner().products.into_iter().map(|p| p.id).collect())
    }
}

struct RecommendationServiceImpl {
    catalog: TonicCatalogClient,
}

#[tonic::async_trait]
impl RecommendationService for RecommendationServiceImpl {
    async fn list_recommendations(
        &self,
        request: Request<ListRecommendationsRequest>,
    ) -> Result<Response<ListRecommendationsResponse>, Status> {
        core::list_recommendations(&self.catalog, request.get_ref())
            .await
            .map(Response::new)
    }
}

// WP-J1/H3: one source for both targets. current_thread on both — wasip2 has no
// threads, and the schedule has to be identical for the comparison to be fair.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .context("invalid PORT")?;
    let addr = format!("0.0.0.0:{}", port);

    let catalog_addr = std::env::var("PRODUCT_CATALOG_SERVICE_ADDR")
        .unwrap_or_else(|_| "localhost:3550".to_string());
    // Resolve here, not in the connector: hyper-util's GaiResolver runs the
    // lookup on tokio's blocking pool, and wasip2 cannot spawn a thread. An IP
    // literal makes HttpConnector skip the resolver entirely. A ClusterIP is
    // stable for the life of the Service; if the lookup fails the pod restarts.
    let catalog_sock = std::net::ToSocketAddrs::to_socket_addrs(&catalog_addr)
        .ok()
        .and_then(|mut it| it.next())
        .with_context(|| format!("cannot resolve {}", catalog_addr))?;
    let service = RecommendationServiceImpl {
        catalog: TonicCatalogClient {
            channel: Endpoint::from_shared(format!("http://{}", catalog_sock))
                .context("invalid PRODUCT_CATALOG_SERVICE_ADDR")?
                .connect_lazy(),
        },
    };

    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .context("bind failed")?;

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<RecommendationServiceServer<RecommendationServiceImpl>>()
        .await;

    println!("RecommendationService listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(RecommendationServiceServer::new(service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await
        .context("gRPC server failed")?;

    Ok(())
}
