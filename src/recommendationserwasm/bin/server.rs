use anyhow::{Context, Result};
use tonic::transport::{Channel, Endpoint, Server};
use tonic::{Request, Response, Status};

pub mod hipstershop {
    tonic::include_proto!("hipstershop");
}

#[path = "../src/core.rs"]
mod core;

use core::CatalogClient;
use hipstershop::product_catalog_service_client::ProductCatalogServiceClient;
use hipstershop::recommendation_service_server::{RecommendationService, RecommendationServiceServer};
use hipstershop::{Empty, ListRecommendationsRequest, ListRecommendationsResponse};

// ---------------------------------------------------------------------------
// Native catalog adapter — one tonic channel for the process lifetime, as the
// Python service holds one grpc channel. build_transport(false) means no
// generated connect() shortcut, so the channel comes from Endpoint.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// gRPC service — delegates entirely to core logic
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port)
        .parse()
        .context("invalid listen address")?;

    let catalog_addr = std::env::var("PRODUCT_CATALOG_SERVICE_ADDR")
        .unwrap_or_else(|_| "localhost:3550".to_string());
    let service = RecommendationServiceImpl {
        catalog: TonicCatalogClient {
            channel: Endpoint::from_shared(format!("http://{}", catalog_addr))
                .context("invalid PRODUCT_CATALOG_SERVICE_ADDR")?
                .connect_lazy(),
        },
    };

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<RecommendationServiceServer<RecommendationServiceImpl>>()
        .await;

    println!("RecommendationService listening on {}", addr);

    Server::builder()
        .add_service(health_service)
        .add_service(RecommendationServiceServer::new(service))
        .serve(addr)
        .await
        .context("gRPC server failed")?;

    Ok(())
}
