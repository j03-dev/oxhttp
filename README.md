# OxHTTP

*OxHTTP* is Python HTTP server library build in Rust

example:

```python
from oxhttp import HttpServer, get, Router

router = Router()

router.route(get("/", lambda: "Welcome to OxHTTP!"))
router.route(get("/hello/<name>", lambda name: ({"message": f"Hello, {name}!"}, 200)))

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
- [ ] middleware
- [ ] app data