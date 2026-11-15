"""Publish verified ML Studio release notes through the existing Discord bot."""

import re
import runpy
import textwrap
from pathlib import Path

GUILD = "1567546974591258644"
CHANNEL = "1568551545954500738"


def bot_api(source_sync):
    # Reuse the machine's existing DPAPI credential loader. No token in Git.
    return runpy.run_path(str(Path(source_sync).with_name("discord-migration-work.py")))["api"]


def payloads(version, notes, download):
    title = f"ML Studio changelog | Build {version}"
    heading = re.sub(r"\s*[\u00b7|]\s*public build \d+\s*$", "", notes["title"])
    date = notes.get("date", f"{str(version)[:4]}-{str(version)[4:6]}-{str(version)[6:8]}")
    body = f"**{heading}**\n{date}\n\n" + "\n".join(f"- {line}" for line in notes["changes"])
    parts = textwrap.wrap(body, 3900, replace_whitespace=False, break_on_hyphens=False)
    for index, part in enumerate(parts, 1):
        marker = f"ML Studio build {version} ({index}/{len(parts)})"
        yield {
            "content": f"[Download ML Studio](<{download}>)",
            "embeds": [{"title": title, "description": part, "footer": {"text": marker}}],
            "allowed_mentions": {"parse": []},
            "nonce": f"ml{version}-{index}",
            "enforce_nonce": True,
        }


def publish(api, version, notes, download):
    channel = api(f"channels/{CHANNEL}")
    if channel.get("guild_id") != GUILD or channel.get("type") != 0:
        raise RuntimeError("Discord changelog destination could not be verified")
    bot_id = api("users/@me")["id"]
    route = f"channels/{CHANNEL}/messages"
    recent = api(f"{route}?limit=100")
    links = []
    for payload in payloads(version, notes, download):
        expected = payload["embeds"][0]

        def matches(message):
            return message["author"]["id"] == bot_id and any(
                embed.get("footer") == expected["footer"]
                and embed.get("title") == expected["title"]
                and embed.get("description") == expected["description"]
                for embed in message["embeds"]
            )

        message = next((item for item in recent if matches(item)), None)
        if message is None:
            message = api(route, "POST", payload)
        message_id = message["id"]
        if not matches(api(f"{route}/{message_id}")):
            raise RuntimeError(f"Discord changelog verification failed for message {message_id}")
        links.append(f"https://discord.com/channels/{GUILD}/{CHANNEL}/{message_id}")
    return {"version": version, "messages": links}
