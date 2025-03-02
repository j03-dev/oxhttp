def extract_fields(cls):
    import inspect
    from typing import get_type_hints

    fields = {}

    try:
        type_hints = get_type_hints(cls)
    except (TypeError, AttributeError):
        return str(cls).replace("<class '", "").replace("'>", "")

    for field_name, field_type in type_hints.items():
        if field_type in (str, int, float, bool, list, dict):
            fields[field_name] = field_type.__name__
        elif hasattr(field_type, "__origin__"):
            fields[field_name] = str(field_type)
        elif inspect.isclass(field_type):
            fields[field_name] = extract_fields(field_type)
        else:
            fields[field_name] = str(field_type)

    return fields
