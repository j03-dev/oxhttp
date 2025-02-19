from jwt import decode, encode, ExpiredSignatureError, InvalidTokenError
from oxhttp import HttpServer, Router, get, post, Status, Response, Request
from typing import Callable

SECRET = "8b78e057cf6bc3e646097e5c0277f5ccaa2d8ac3b6d4a4d8c73c7f6af02f0ccd"


def create_jwt(user_id: int) -> str:
    payload = {"user_id": user_id}
    return encode(payload, SECRET, algorithm="HS256")


def decode_jwt(token: str):
    try:
        return decode(token, SECRET, algorithms=["HS256"])
    except ExpiredSignatureError:
        return None
    except InvalidTokenError:
        return None


def login(cred: dict):
    if cred.get("username") == "admin" and cred.get("password") == "password":
        token = create_jwt(user_id=1)
        return {"token": token}
    return Status.UNAUTHORIZED()


def user_info(user_id) -> Response:
    return {"user_id": user_id}


def jwt_middleware(request: Request, next: Callable, **kwargs):
    headers = request.headers()
    token = headers.get("Authorization", "").replace("Bearer ", "")

    if token:
        payload = decode_jwt(token)
        if payload:
            kwargs["user_id"] = payload["user_id"]
            return next(**kwargs)

    return Status.UNAUTHORIZED()


sec_router = Router()
sec_router.middleware(jwt_middleware)
sec_router.route(get("/me", user_info))

pub_router = Router()
pub_router.route(post("/login", login))
pub_router.route(get("/hello/<name>", lambda name: f"Hello {name}"))

server = HttpServer(("127.0.0.1", 5555))
server.attach(sec_router)
server.attach(pub_router)

if __name__ == "__main__":
    server.run()
