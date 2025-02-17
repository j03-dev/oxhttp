from oxhttp.oxhttp import HttpServer
from oxhttp.oxhttp import routing

router = routing.Router()
router.route(routing.get("/hello/<name>", lambda name: (f"Hello {name}", 200)))

app = HttpServer(("127.0.0.1", 5555))
app.attach(router)

if __name__ == "__main__":
    app.run()
