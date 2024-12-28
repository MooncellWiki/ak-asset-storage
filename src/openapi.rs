use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
#[derive(OpenApi)]
#[openapi()]
pub struct BaseOpenApi;

impl BaseOpenApi {
    #[must_use]
    pub fn router<S>() -> OpenApiRouter<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        OpenApiRouter::with_openapi(Self::openapi())
    }
}
