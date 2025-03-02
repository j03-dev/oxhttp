from oxhttp import get


class Dog:
    name: str


class User:
    name: str
    dog: Dog


def handler(user: User):
    pass


route = get("/test", handler)

print(route)
