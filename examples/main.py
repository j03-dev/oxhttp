import sqlite3
from utils import hash_password, create_jwt, check_password
from middlewares import logger, jwt_middleware

from oxhttp import (
    HttpServer,
    Response,
    Router,
    Status,
    get,
    post,
    static_files,
)


@post("/register", data="user_input")
def register(user_input, app_data):
    conn = app_data.conn
    username = user_input.get("username")
    password = user_input.get("password")

    if not username or not password:
        return Status.BAD_REQUEST

    hashed_password = hash_password(password)

    try:
        conn.execute(
            "insert into user (username, password) values (?, ?)",
            (username, hashed_password),
        )
        conn.commit()
        return Status.CREATED
    except sqlite3.IntegrityError:
        return Status.CONFLICT


@post("/login", data="cred")
def login(cred: dict, app_data):
    conn = app_data.conn
    username = cred.get("username")
    password = cred.get("password")

    cursor = conn.execute(
        "select id, password from user where username=?",
        (username,),
    )
    user = cursor.fetchone()

    if user and check_password(user[1], password):
        token = create_jwt(user_id=user[0])
        return {"token": token}

    return Status.UNAUTHORIZED


@get("/hello/{name}")
def hello_world(name):
    return f"Hello {name}"


@get("/me")
def user_info(user_id: int, app_data) -> Response:
    result = app_data.conn.execute("select * from user where id=?", (user_id,))
    return Response(Status.OK, {"user": result.fetchone()})


class AppData:
    conn = sqlite3.connect("database.db")


pub_router = Router()
pub_router.middleware(logger)
pub_router.routes([hello_world, login, register])
pub_router.route(static_files("./static", "static"))


sec_router = Router()
sec_router.route(user_info)
sec_router.middleware(logger)
sec_router.middleware(jwt_middleware)


server = HttpServer(("127.0.0.1", 5555))
server.app_data(AppData)
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
