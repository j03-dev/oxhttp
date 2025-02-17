from oxhttp import HttpServer, Router, get

def user_info():
    """Fetch user information."""
    return {"phone_number": 00000}, 200

def auth():
    """Authentication middleware logic."""
    return "Unauthorized", 401

# Secure router with authentication
sec_router = Router()
sec_router.middleware(auth)
sec_router.route(get("/me", user_info))

# Public router
pub_router = Router()
pub_router.route(get("/hello/<name>", lambda name: (f"Hello {name}", 200)))

# Create and configure the HTTP server
server = HttpServer(("127.0.0.1", 5555))
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
