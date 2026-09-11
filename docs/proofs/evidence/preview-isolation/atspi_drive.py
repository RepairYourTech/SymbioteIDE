import sys
import time
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def find_node(node, role_name, name_contains, depth=0):
    try:
        role = node.get_role().value_name.replace("ROLE_", "")
        name = node.get_name() or ""
    except Exception:
        return None
    if role_name in role and name_contains.lower() in name.lower():
        return node
    if depth > 20:
        return None
    try:
        count = node.get_child_count()
    except Exception:
        return None
    for i in range(count):
        try:
            child = node.get_child_at_index(i)
        except Exception:
            continue
        found = find_node(child, role_name, name_contains, depth + 1)
        if found is not None:
            return found
    return None

def find_app(target="symbiote-desktop"):
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if target in (app.get_name() or "").lower():
            return app
    raise SystemExit(f"application {target} not found")

def click_button(name):
    app = find_app()
    button = find_node(app, "BUTTON", name)
    if button is None:
        raise SystemExit(f"button {name!r} not found")
    button.do_action(0)
    print(f"clicked {name!r}")

def read_log():
    app = find_app()
    node = find_node(app, "SECTION", "")
    # The log lives in the LAST section; scan all text-capable nodes.
    texts = []
    def collect(node, depth=0):
        try:
            role = node.get_role().value_name
            if role in ("ROLE_SECTION", "ROLE_PARAGRAPH", "ROLE_DOCUMENT_WEB"):
                try:
                    text = node.get_text(0, -1)
                    if text and text.strip():
                        texts.append((role, text))
                except Exception:
                    pass
        except Exception:
            pass
        try:
            count = node.get_child_count()
        except Exception:
            return
        for i in range(count):
            try:
                collect(node.get_child_at_index(i), depth + 1)
            except Exception:
                pass
    collect(app)
    return texts

command = sys.argv[1]
if command == "click":
    click_button(sys.argv[2])
elif command == "log":
    for role, text in read_log():
        print(f"--- {role} ---")
        print(text)
elif command == "marker":
    # The execution discriminator (#54): the hostile script appends its
    # marker as a SECOND, standalone DOM node when it runs — the inert
    # page shows it exactly once, inside the escaped literal. Also
    # asserts the isolation self-check verdict is present.
    texts = [text for _, text in read_log()]
    blob = "\n".join(texts)
    occurrences = blob.count("WORKER_SCRIPT_EXECUTED")
    assert occurrences == 1, (
        f"the hostile script's marker must appear exactly once "
        f"(escaped literal), got {occurrences}"
    )
    assert "Isolation self-check: OK: app command rejected by the app ACL" in blob, (
        "the isolation self-check verdict must be present"
    )
    print("INERTNESS_OK: marker once (inside the literal), probe verdict present")
