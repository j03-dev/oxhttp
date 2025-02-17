from oxhttp import HttpServer, Router, get


def hello_world(name: str):
    {"message": f"Hello {name}"}, 200


router = Router()
router.route(get("/hello/<name>", hello_world))

app = HttpServer(("127.0.0.1", 5555))
app.attach(router)

if __name__ == "__main__":
    app.run()
