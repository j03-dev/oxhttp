from oxhttp.oxhttp import get, HttpServer

app = HttpServer(("127.0.0.1", 5555))
app.route(get("/hello/<name>", lambda name: (f"Hello {name}", 200)))

if __name__ == "__main__":
    app.run()
