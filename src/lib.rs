use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    dotenv::dotenv().ok();
    Response::ok("Hello, World!")
}
