# OxHTTP

*OxHTTP* is Python HTTP server library build in Rust

example:

```python
from oxhttp import HttpServer, get

app = HttpServer(("127.0.0.1", 5555))

app.route(get("/", lambda: "Welcome to OxHTTP!"))
@app.route(get("/hello/<name>", lambda name: ({"message": f"Hello, {name}!"}, 200)))

if __name__ == "__main__":
    app.run()
```