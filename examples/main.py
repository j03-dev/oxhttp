from oxhttp import HttpServer, Router, get, Status, Response


def user_info() -> Response:
    """Fetch user information."""
    return Response(Status.OK(), "Hello")


def auth() -> Response:
    """Authentication middleware logic."""
    return Response(Status.UNAUTHORIZED(), "UnAuthorized")


# Secure router with authentication
sec_router = Router()
sec_router.middleware(auth)
sec_router.route(get("/me", user_info))

# Public router
pub_router = Router()
pub_router.route(
    get("/hello/<name>", lambda name: Response(Status.OK(), f"Hello {name}"))
)

# Create and configure the HTTP server
server = HttpServer(("127.0.0.1", 5555))
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
