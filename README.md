# OxHTTP

*OxHTTP* is Python HTTP server library build in Rust

example:

```python
from oxhttp import HttpServer, get, Router, Status, Response

router = Router()

router.route(get("/", lambda request: Response(Status.OK(), "Welcome to OxHTTP!")))
router.route(
    get("/hello/<name>", lambda request, name: Response(Status.Ok(), {"message": f"Hello, {name}!"}))
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
- [ ] use tokio::net::Listener
- [x] middleware
- [ ] app data
- [x] pass request in handler
