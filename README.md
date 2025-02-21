# OxHTTP

_OxHTTP_ is Python HTTP server library build in Rust

example:

```python
from oxhttp import HttpServer, get, Router, Status, Response

router = Router()

router.route(get("/", lambda: Response(Status.OK(), "Welcome to OxHTTP!")))
router.route(
    get("/hello/<name>", lambda, name: Response(Status.Ok(), {"message": f"Hello, {name}!"}))
)

app = HttpServer(("127.0.0.1", 5555))
app.attach(router)

if __name__ == "__main__":
    app.run()
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
