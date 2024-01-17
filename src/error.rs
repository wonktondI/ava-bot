use async_trait::async_trait;
use salvo::http::StatusCode;
use salvo::{Depot, Request, Response, Writer};

pub struct AppError(anyhow::Error);

#[async_trait]
impl Writer for AppError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        let err_msg = format!("Something went wrong:{}", self.0);
        res.render(err_msg);
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
