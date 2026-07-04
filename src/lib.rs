use worker::*;

#[event(fetch)]
async fn fetch(
    _req: Request,
    _env: Env,
    _ctx: Context,
) -> Result<Response> {
    let socket = Socket::builder()
        .connect("example.com", 80)?;
        
    Response::ok("Hello World!")
}