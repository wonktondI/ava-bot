use askama::Template;
use salvo::http::cookie::Cookie;
use salvo::prelude::Text;
use salvo::{handler, Request, Response};
use uuid::Uuid;

pub const COOKIE_NAME: &str = "device_id";

#[derive(Debug, Template)]
#[template(path = "index.html.j2")]
struct IndexTemplate {}

#[handler]
pub async fn index_page(req: &mut Request, res: &mut Response) {
    let device_id_cookie = req.cookies().get(COOKIE_NAME).cloned().unwrap_or_else(|| {
        let new_id = Uuid::new_v4().to_string();
        Cookie::build((COOKIE_NAME, new_id))
            .path("/")
            .secure(true)
            .permanent()
            .build()
    });

    let index_template = IndexTemplate {};
    res.add_cookie(device_id_cookie)
        .render(Text::Html(index_template.render().unwrap()));
}
