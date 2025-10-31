#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "httpx>=0.28.1,<0.29.0",
#     "rich>=14.1.0,<15.0.0",
#     "typer>=0.20.0,<0.21.0",
# ]
# ///

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from enum import Enum, auto
from itertools import pairwise
from typing import Annotated
from urllib.parse import urljoin

import httpx
from rich.console import Console
from rich.table import Column, Table
from sklearn.linear_model import LinearRegression
from typer import Option, run


class WorkingMode(Enum):
    CHARGING = auto()
    DISCHARGING = auto()
    IDLING = auto()


@dataclass(slots=True, kw_only=True)
class Delta:
    duration: timedelta = timedelta()
    charge: float = 0.0
    imported: float = 0.0
    exported: float = 0.0

    def __add__(self, other: Delta) -> Delta:
        return Delta(
            duration=(self.duration + other.duration),
            charge=(self.charge + other.charge),
            imported=(self.imported + other.imported),
            exported=(self.exported + other.exported),
        )

    @property
    def total_hours(self) -> float:
        return self.duration.total_seconds() / 3600.0

    @property
    def is_importing(self) -> bool:
        return self.imported >= 0.001

    @property
    def is_exporting(self) -> bool:
        return self.exported >= 0.001

    @property
    def as_parasitic_load(self) -> float:
        return (self.exported - self.imported - self.charge) / self.total_hours

    def charging_efficiency(self, parasitic_load: float) -> float:
        return self.charge / (self.imported - self.exported - parasitic_load * self.total_hours)

    def discharging_efficiency(self, parasitic_load: float) -> float:
        return (self.imported - self.exported) / (self.charge - parasitic_load * self.total_hours)


@dataclass(frozen=True, slots=True)
class State:
    timestamp: datetime
    residual_energy: float
    total_import: float
    total_export: float

    def __sub__(self, rhs: State) -> Delta:
        return Delta(
            duration=(self.timestamp - rhs.timestamp),
            charge=(self.residual_energy - rhs.residual_energy),
            imported=(self.total_import - rhs.total_import),
            exported=(self.total_export - rhs.total_export),
        )


def fetch_states(*, home_assistant_url: str, authorization_token: str, entity_id: str) -> Iterable[State]:
    now = datetime.now(UTC)
    since = now - timedelta(days=365.25)

    response = httpx.get(
        urljoin(home_assistant_url, f"api/history/period/{since.isoformat()}"),
        params=[
            ("filter_entity_id", entity_id),
            ("end_time", now.isoformat()),
        ],
        headers={"Authorization": f"Bearer {authorization_token}"},
    )
    response.raise_for_status()

    for state in response.json()[0]:
        if isinstance(residual_energy := state["attributes"].get("custom_battery_residual_energy"), float):
            yield State(
                timestamp=datetime.fromisoformat(state["last_changed"]),
                residual_energy=residual_energy,
                total_import=float(state["attributes"]["custom_battery_energy_import"]),
                total_export=float(state["attributes"]["custom_battery_energy_export"]),
            )


def differentiate(states: Iterable[State]) -> Iterable[Delta]:
    for from_state, to_state in pairwise(states):
        if (
            (from_state.timestamp < to_state.timestamp)
            and (from_state.total_import <= to_state.total_import)
            and (from_state.total_export <= to_state.total_export)
        ):
            yield to_state - from_state


def main(
    entity_id: Annotated[
        str,
        Option(help="Home Assistant entity ID with the following attributes: `custom_battery_residual_energy`, `custom_battery_energy_import`, and `custom_battery_energy_export`."),
    ],
    authorization_token: Annotated[str, Option(help="Home Assistant authorization token.")],
    home_assistant_url: Annotated[str, Option(help="Home Assistant URL.")] = "http://localhost:8123",
) -> None:
    """
    Estimate battery performance parameters using the historical sensor data from Home Assistant.
    """

    states = list(fetch_states(
        home_assistant_url=home_assistant_url,
        authorization_token=authorization_token,
        entity_id=entity_id,
    ))
    deltas = list(differentiate(states))

    charging_stats = Delta()
    discharging_stats = Delta()
    idling_stats = Delta()
    mixed_stats = Delta()

    for delta in deltas:
        if delta.is_importing:
            if delta.is_exporting:
                mixed_stats += delta
            else:
                # Importing without exporting must be charging:
                charging_stats += delta
        else:
            if delta.is_exporting:
                # Exporting without importing must be discharging:
                discharging_stats += delta
            elif delta.charge <= 0.0:
                # No exporting and no importing with no charge gained:
                idling_stats += delta
            else:
                # No importing nor exporting, but gained charge:
                charging_stats += delta

    console = Console()

    table = Table(
        Column(header=f"{len(states)} states ({len(deltas)} delta's)"),
        Column(header="Duration"),
        Column(header="Import"),
        Column(header="Export"),
        Column(header="Net charge"),
        title="Cumulative statistics",
        title_justify="left",
    )
    for title, stats in [
        ("Charging", charging_stats),
        ("Discharging", discharging_stats),
        ("Idling", idling_stats),
        ("Ignored", mixed_stats),
    ]:
        table.add_row(title, f"{stats.duration}", f"{stats.imported:.3f} kWh", f"{stats.exported:.3f} kWh", f"{stats.charge:+.3f} kWh")
    console.print(table)

    parasitic_load = idling_stats.as_parasitic_load
    charging_efficiency = charging_stats.charging_efficiency(parasitic_load)
    discharging_efficiency = discharging_stats.discharging_efficiency(parasitic_load)

    table = Table(
        Column(style="bold"),
        title="Estimated battery parameters",
        show_header=False,
        title_justify="left",
    )
    table.add_row("Parasitic load", f"{parasitic_load:.3f} kW")
    table.add_row("Charging efficiency", f"{charging_efficiency:.3f}")
    table.add_row("Discharging efficiency", f"{discharging_efficiency:.3f}")
    table.add_row("Round-trip efficiency", f"{charging_efficiency * discharging_efficiency:.3f}")
    console.print(table)

    # Charging: 1.087 kW / 1.192 kW = 0.912
    # Discharging: 0.8 kW / 0.856 kW = 0.935
    # Roundtrip: 0.912 * 0.935 = 0.853


if __name__ == "__main__":
    run(main)
