import sqlite3
import bcrypt
from jwt import decode, encode, ExpiredSignatureError, InvalidTokenError
from oxhttp import HttpServer, Router, get, post, static_files, Status, Response, Request
from typing import Callable

SECRET = "8b78e057cf6bc3e646097e5c0277f5ccaa2d8ac3b6d4a4d8c73c7f6af02f0ccd"

# database_connection = sqlite3.connect("database.db")
# database_connection.execute(
#     """
#     create table if not exists user (
#         id integer primary key autoincrement,
#         username varchar(255) unique,
#         password varchar(255)
#     );
#     """
# )


class AppData:
    conn = sqlite3.connect("database.db")


def create_jwt(user_id: int) -> str:
    payload = {"user_id": user_id}
    return encode(payload, SECRET, algorithm="HS256")


def decode_jwt(token: str):
    try:
        return decode(token, SECRET, algorithms=["HS256"])
    except (ExpiredSignatureError, InvalidTokenError):
        return None


def hash_password(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()


def check_password(hashed_password: str, password: str) -> bool:
    return bcrypt.checkpw(password.encode(), hashed_password.encode())


def register(cred: dict, app_data: AppData):
    conn = app_data.conn
    username = cred.get("username")
    password = cred.get("password")

    if not username or not password:
        return Status.BAD_REQUEST()

    hashed_password = hash_password(password)

    try:
        conn.execute(
            "insert into user (username, password) values (?, ?)",
            (username, hashed_password),
        )
        conn.commit()
        return Status.CREATED()
    except sqlite3.IntegrityError:
        return Status.CONFLICT()


def login(cred: dict, app_data: AppData):
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

    return Status.UNAUTHORIZED()


def user_info(user_id: int, app_data: AppData) -> Response:
    result = app_data.conn.execute("select * from user where id=?", (user_id,))
    return Response(Status.OK(), {"user": result.fetchone()})


def jwt_middleware(request: Request, next: Callable, **kwargs):
    headers = request.headers()
    token = headers.get("Authorization", "").replace("Bearer ", "")

    if token:
        if payload := decode_jwt(token):
            kwargs["user_id"] = payload["user_id"]
            return next(**kwargs)
    return Status.UNAUTHORIZED()


sec_router = Router()
sec_router.middleware(jwt_middleware)
sec_router.route(get("/me", user_info))

pub_router = Router()
pub_router.route(post("/login", login))
pub_router.route(post("/register", register))
pub_router.route(get("/hello/<name>", lambda name: f"Hello {name}"))

pub_router.route(static_files("./static", "static"))

server = HttpServer(("127.0.0.1", 5555))
server.app_data(AppData)
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
