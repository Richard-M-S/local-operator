use axum::{extract::Path, Json};

use crate::{
    domains::catalog::{catalog_response, find_domain, DomainCatalogResponse, DomainDescriptor},
    error::AppError,
};

pub async fn list() -> Json<DomainCatalogResponse> {
    Json(catalog_response())
}

pub async fn get(Path(domain): Path<String>) -> Result<Json<DomainDescriptor>, AppError> {
    find_domain(&domain)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("domain {} not found", domain)))
}
