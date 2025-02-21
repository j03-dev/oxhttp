# OxHTTP

_OxHTTP_ is Python HTTP server library build in Rust - a fast, safe and feature-rich HTTP server implementation.

## Features

- Routing with path parameters
- Middleware support
- Static file serving
- Application state management
- Request/Response handling
- Query string parsing

## Basic Example

```python
from oxhttp import HttpServer, get, Router, Status, Response

router = Router()

router.route(get("/", lambda: Response(Status.OK(), "Welcome to OxHTTP!")))
router.route(
    get("/hello/<name>", lambda name: Response(Status.OK(), {"message": f"Hello, {name}!"}))
)

app = HttpServer(("127.0.0.1", 5555))
app.attach(router)

if __name__ == "__main__":
    app.run()
```

## Middleware Example

```python
def auth_middleware(request, next, **kwargs):
    if "Authorization" not in request.headers():
        return Status.UNAUTHORIZED()
    return next(**kwargs)

router = Router()
router.middleware(auth_middleware)
router.route(get("/protected", lambda: "This is protected!"))
```

## Static Files

```python
router = Router()
router.route(static_files("./static", "static"))
# Serves files from ./static directory at /static URL path
```

## Application State

```python
class AppState:
    def __init__(self):
        self.counter = 0

app = HttpServer(("127.0.0.1", 5555))
app.app_data(AppState)

def handler(app_data):
    app_data.counter += 1
    return {"count": app_data.counter}

router = Router()
router.route(get("/count", handler))
```

Todo:

- [x] Handler
- [x] HttpResponse
- [x] Routing
- [ ] use tokio::net::Listener
- [x] middleware
- [x] app data
- [x] pass request in handler
- [x] serve static file
- [ ] templating
- [x] query uri
