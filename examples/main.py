from oxhttp import HttpServer, Router, get, Status, Response, Context
from typing import Callable


def user_info(context: Context, id) -> Response:
    user = context.get_variable("user")
    return Response(Status.OK(), f"Hello, {user}! id {id}")


def auth(context: Context, handler: Callable, **kwargs) -> Response:
    context.set_variable("user", "Authenticated User")
    return handler(context, **kwargs)


sec_router = Router()
sec_router.middleware(auth)
sec_router.route(get("/me/<id>", user_info))

pub_router = Router()
pub_router.route(
    get(
        "/hello/<name>",
        lambda context, name: Response(Status.OK(), f"Hello {name}"),
    )
)

server = HttpServer(("127.0.0.1", 5555))
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
