#!/usr/bin/env python3
"""
PokéDash - Companion Python Terminal Client
Connects to the Rust PokéDash backend API or interacts directly.
"""

import sys
import argparse
import json
import urllib.request
import urllib.error

API_BASE = "http://localhost:8080/api"

try:
    from rich.console import Console
    from rich.table import Table
    from rich.panel import Panel
    from rich.progress import Progress, BarColumn, TextColumn
    console = Console()
except ImportError:
    console = None


def print_msg(msg, style=None):
    if console:
        console.print(msg, style=style)
    else:
        print(msg)


def api_request(endpoint, method="GET", data=None):
    url = f"{API_BASE}{endpoint}"
    req = urllib.request.Request(url, method=method)
    req.add_header("Content-Type", "application/json")
    body = json.dumps(data).encode("utf-8") if data else None

    try:
        with urllib.request.urlopen(req, data=body, timeout=5) as response:
            return json.loads(response.read().decode("utf-8")), None
    except urllib.error.HTTPError as e:
        try:
            err_json = json.loads(e.read().decode("utf-8"))
            return None, err_json.get("error", str(e))
        except Exception:
            return None, str(e)
    except Exception as e:
        return None, f"Could not connect to PokéDash backend at {API_BASE}. Make sure 'pokedash serve' is running! ({e})"


def cmd_hunt():
    res, err = api_request("/hunt", method="POST")
    if err:
        print_msg(f"[bold yellow]⏳ {err}[/bold yellow]")
        return

    encounter = res["encounter"]
    p = encounter["pokemon"]
    already_caught = res.get("already_caught", False)

    badge = "[green][CAUGHT IN DEX][/green]" if already_caught else "[bold yellow][NEW SPECIES! ✨][/bold yellow]"

    if console:
        types_str = " / ".join(p["types"])
        stats_str = f"HP: {p['stats']['HP']} | Atk: {p['stats']['Atk']} | Def: {p['stats']['Def']} | Spe: {p['stats']['Spe']} | BST: {p['stats']['Total']}"
        panel_content = f"""[bold yellow]A wild {p['name'].upper()} appeared! {badge}[/bold yellow]
#{p['id']:03d} • Rarity: [magenta]{p['rarity']}[/magenta] • Types: [cyan]{types_str}[/cyan]
Abilities: {', '.join(p['abilities'])}

{stats_str}

Use [bold cyan]python pokedash_cli.py catch pb[/bold cyan] to throw a Poké Ball!"""
        console.print(Panel(panel_content, title="⚔️ WILD ENCOUNTER", border_style="cyan"))
    else:
        print(f"Wild {p['name']} appeared! Rarity: {p['rarity']}, Types: {'/'.join(p['types'])}")


def cmd_catch(ball):
    res, err = api_request("/catch", method="POST", data={"ball": ball})
    if err:
        print_msg(f"[bold red]❌ {err}[/bold red]")
        return

    if res["success"]:
        print_msg(f"[bold green]✨ {res['message']}[/bold green]")
        print_msg(f"Remaining {res['ball']}s: [yellow]{res['balls_remaining']}[/yellow]")
        if res.get("leveled_up"):
            print_msg(f"[bold yellow]🎉 LEVEL UP! You are now Trainer Level {res['new_level']}![/bold yellow]")
    else:
        print_msg(f"[bold red]❌ {res['message']}[/bold red]")
        print_msg(f"Remaining {res['ball']}s: [yellow]{res['balls_remaining']}[/yellow]")


def cmd_pokedex():
    res, err = api_request("/pokedex?limit=50")
    if err:
        print_msg(f"[bold red]❌ {err}[/bold red]")
        return

    pokemon = res.get("pokemon", [])
    if not pokemon:
        print_msg("Your Pokédex is currently empty. Run 'hunt' to catch some Pokémon!")
        return

    if console:
        table = Table(title=f"YOUR POKÉDEX ({res['total']} Caught)")
        table.add_column("#", style="dim", width=6)
        table.add_column("Name", style="bold cyan")
        table.add_column("Type", style="green")
        table.add_column("Rarity", style="magenta")
        table.add_column("BST", style="yellow")

        for p in pokemon:
            table.add_row(
                f"{p['id']:03d}",
                p["name"],
                "/".join(p["types"]),
                p["rarity"],
                str(p["stats"]["Total"]),
            )
        console.print(table)
    else:
        for p in pokemon:
            print(f"#{p['id']:03d} {p['name']} ({p['rarity']})")


def cmd_stats():
    res, err = api_request("/player")
    if err:
        print_msg(f"[bold red]❌ {err}[/bold red]")
        return

    p = res["player"]
    s = p["stats"]

    if console:
        console.print(f"\n  [bold cyan]{p['username']}[/bold cyan]")
        console.print(f"  Level: [bold]{res['level']}[/bold]   •   Poké Dollars: [yellow]₽ {p['currency']:,}[/yellow]   •   Discount: [green]{res['discount_pct']}%[/green]")
        console.print(f"  Pokédex Unique: [green]{res['unique_count']} / 1025[/green]   •   Total Hunts: [cyan]{s['total_hunts']}[/cyan]")
        console.print(f"  Successful Catches: [green]{s['successful_catches']}[/green]   •   Failed: [red]{s['failed_catches']}[/red]\n")
    else:
        print(f"Level: {res['level']}, Balance: ₽{p['currency']}, XP: {p['xp']}")


def cmd_inventory():
    res, err = api_request("/player")
    if err:
        print_msg(f"[bold red]❌ {err}[/bold red]")
        return

    p = res["player"]
    balls = p["balls"]

    if console:
        table = Table(title=f"BAG INVENTORY — ₽ {p['currency']:,}")
        table.add_column("Poké Ball Type", style="bold white")
        table.add_column("In Bag", style="bold yellow")

        for b, count in balls.items():
            if count > 0:
                table.add_row(b.replace("_", " ").title(), str(count))
        console.print(table)
    else:
        for b, count in balls.items():
            if count > 0:
                print(f"{b}: {count}")


def cmd_store():
    res, err = api_request("/store")
    if err:
        print_msg(f"[bold red]❌ {err}[/bold red]")
        return

    if console:
        table = Table(title=f"POKÉ MART (Balance: ₽ {res['currency']:,} • Discount: {res['discount_pct']}%)")
        table.add_column("Item", style="bold white")
        table.add_column("Alias", style="cyan")
        table.add_column("Price", style="bold yellow")
        table.add_column("Owned", style="green")
        table.add_column("Req. Lv", style="dim")
        table.add_column("Description")

        for item in res["items"]:
            table.add_row(
                item["name"],
                item["alias"],
                f"₽ {item['discounted_price']:,}",
                str(item["owned"]),
                f"Lv {item['unlock_level']}",
                item["description"],
            )
        console.print(table)
    else:
        for item in res["items"]:
            print(f"{item['name']} ({item['alias']}): ₽{item['discounted_price']}")


def main():
    parser = argparse.ArgumentParser(description="PokéDash Companion Python CLI")
    subparsers = parser.add_subparsers(dest="command")

    subparsers.add_parser("hunt", help="Hunt for a wild Pokémon")

    catch_p = subparsers.add_parser("catch", help="Catch a wild Pokémon with a ball")
    catch_p.add_argument("ball", help="Ball to throw (e.g. pb, gb, ub, net, dive, etc.)")

    subparsers.add_parser("dex", help="View your Pokédex")
    subparsers.add_parser("stats", help="View trainer stats")
    subparsers.add_parser("inv", help="View inventory")
    subparsers.add_parser("store", help="View Poké Mart store")

    args = parser.parse_args()

    if args.command == "hunt":
        cmd_hunt()
    elif args.command == "catch":
        cmd_catch(args.ball)
    elif args.command == "dex":
        cmd_pokedex()
    elif args.command == "stats":
        cmd_stats()
    elif args.command == "inv":
        cmd_inventory()
    elif args.command == "store":
        cmd_store()
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
