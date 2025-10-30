#!/usr/bin/env -S uv run --script

# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "httpx>=0.28.1,<0.29.0",
#     "scikit-learn>=1.7.2,<2.0.0",
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
from sklearn.linear_model import LinearRegression
from typer import Option, run


class WorkingMode(Enum):
    CHARGING = auto()
    DISCHARGING = auto()
    IDLING = auto()


@dataclass(slots=True, kw_only=True)
class Delta:
    time: timedelta = timedelta()
    energy: float = 0.0
    imported: float = 0.0
    exported: float = 0.0

    def __post_init__(self) -> None:
        if self.imported < 0.0:
            self.exported -= self.imported
            self.imported = 0.0
        assert self.exported >= 0.0

    @property
    def total_hours(self) -> float:
        return self.time.total_seconds() / 3600.0


@dataclass(frozen=True, slots=True)
class State:
    timestamp: datetime
    residual_energy: float
    total_import: float
    total_export: float

    def __sub__(self, rhs: State) -> Delta:
        return Delta(
            time=(self.timestamp - rhs.timestamp),
            energy=(self.residual_energy - rhs.residual_energy),
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
        if (residual_energy := state["attributes"].get("custom_battery_residual_energy")) is not None:
            yield State(
                timestamp=datetime.fromisoformat(state["last_changed"]),
                residual_energy=residual_energy,
                total_import=state["attributes"]["custom_battery_energy_import"],
                total_export=state["attributes"]["custom_battery_energy_export"],
            )


def differentiate(states: Iterable[State]) -> Iterable[Delta]:
    for from_state, to_state in pairwise(states):
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
    print(f"Fetched {len(states)} states ({len(deltas)} delta's)")

    model = LinearRegression()
    model.fit(
        [[delta.imported, delta.exported] for delta in deltas],
        [delta.energy for delta in deltas],
        sample_weight=[delta.total_hours for delta in deltas],
    )
    parasitic_load = -model.intercept_
    charging_efficiency = model.coef_[0]
    discharging_efficiency = -1.0 / model.coef_[1]

    print(f"Parasitic load: {parasitic_load:.3f} kW")
    print(f"Charging efficiency: {charging_efficiency:.3f}")
    print(f"Discharging efficiency: {discharging_efficiency:.3f}")
    print(f"Round-trip efficiency: {charging_efficiency * discharging_efficiency:.3f}")

    # Charging: 1.087 kW / 1.192 kW = 0.912
    # Discharging: 0.8 kW / 0.856 kW = 0.935
    # Roundtrip: 0.912 * 0.935 = 0.853


if __name__ == "__main__":
    run(main)
