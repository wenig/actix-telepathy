import yaml
import os


def load_settings() -> dict:
    with open("settings.yaml", "r") as f:
        settings = yaml.load(f, yaml.Loader)
    return settings

def to_terminal_args(kwargs: dict) -> str:
    res = ""
    for k, v in kwargs.items():
        res += f"--{k} {v} "
    return res

def main():
    settings = load_settings()["fabric"]["experiment"]
    base_port: int = settings["settings"]["base_port"]
    kwargs = to_terminal_args(settings["kwargs"])

    os.system(f"DECENTFL_BASEPORT={base_port} DECENTFL_BASEHOST=odin01 bash start_decentfl.sh 5 {kwargs}")


if __name__ == "__main__":
    main()
