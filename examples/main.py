from oxhttp import HttpServer, Router, get

router = Router()
router.route(get("/hello/<name>", lambda name: ({"message": f"Hello {name}"}, 200)))

app = HttpServer(("127.0.0.1", 5555))
app.attach(router)

if __name__ == "__main__":
    app.run()
