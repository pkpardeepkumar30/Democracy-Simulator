"""Build compact local city-plan assets from published BBBike OSM extracts."""

from __future__ import annotations

import json
import math
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import osmium
import requests

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CACHE_DIRECTORY = REPOSITORY_ROOT / ".cache" / "city-plans"
OUTPUT_PATH = REPOSITORY_ROOT / "web" / "city-data" / "city-plans.json"
USER_AGENT = "democracy-city-sim-map-import/0.1"

CITIES = [
    {
        "id": "new-delhi-india",
        "city": "New Delhi",
        "country": "India",
        "bbbike": "NewDelhi",
        "bbox": [28.60, 77.18, 28.66, 77.25],
        "zoom": 13,
    },
    {
        "id": "beijing-china",
        "city": "Beijing",
        "country": "China",
        "bbbike": "Beijing",
        "bbox": [39.87, 116.32, 39.95, 116.45],
        "zoom": 12,
    },
    {
        "id": "rio-de-janeiro-brazil",
        "city": "Rio de Janeiro",
        "country": "Brazil",
        "bbbike": "RiodeJaneiro",
        "bbox": [-22.98, -43.25, -22.89, -43.15],
        "zoom": 13,
    },
]

ROAD_CLASSES = {"motorway", "trunk", "primary", "secondary"}
RAIL_CLASSES = {"rail", "subway", "light_rail"}
WATER_CLASSES = {"river", "canal", "stream"}


def squared_distance(
    point: tuple[float, float],
    start: tuple[float, float],
    end: tuple[float, float],
) -> float:
    x, y = start
    dx = end[0] - x
    dy = end[1] - y
    if dx or dy:
        ratio = ((point[0] - x) * dx + (point[1] - y) * dy) / (dx * dx + dy * dy)
        if ratio > 1:
            x, y = end
        elif ratio > 0:
            x += dx * ratio
            y += dy * ratio
    return (point[0] - x) ** 2 + (point[1] - y) ** 2


def simplify(
    points: list[tuple[float, float]], tolerance: float = 1.2
) -> list[tuple[float, float]]:
    if len(points) <= 2:
        return points
    keep = [False] * len(points)
    keep[0] = keep[-1] = True
    stack = [(0, len(points) - 1)]
    threshold = tolerance**2
    while stack:
        first, last = stack.pop()
        farthest = threshold
        index = -1
        for candidate in range(first + 1, last):
            distance = squared_distance(points[candidate], points[first], points[last])
            if distance > farthest:
                farthest = distance
                index = candidate
        if index >= 0:
            keep[index] = True
            stack.extend([(first, index), (index, last)])
    return [point for index, point in enumerate(points) if keep[index]]


def projector(bbox: list[float]):
    south, west, north, east = bbox
    longitude_scale = math.cos(math.radians((south + north) / 2))
    projected_width = (east - west) * longitude_scale
    projected_height = north - south
    scale = min(950 / projected_width, 530 / projected_height)
    width = projected_width * scale
    height = projected_height * scale
    offset_x = (1000 - width) / 2
    offset_y = (600 - height) / 2

    def project(lat: float, lon: float) -> tuple[float, float]:
        return (
            round(offset_x + (lon - west) * longitude_scale * scale, 1),
            round(offset_y + (north - lat) * scale, 1),
        )

    return project


class CityPlanHandler(osmium.SimpleHandler):
    def __init__(self, city: dict[str, Any]) -> None:
        super().__init__()
        self.city = city
        self.features: list[dict[str, Any]] = []
        self.project = projector(city["bbox"])

    def way(self, way: osmium.osm.Way) -> None:
        tags = way.tags
        kind: str | None = None
        feature_class: str | None = None
        if tags.get("highway") in ROAD_CLASSES:
            kind, feature_class = "road", tags.get("highway")
        elif tags.get("railway") in RAIL_CLASSES:
            kind, feature_class = "rail", tags.get("railway")
        elif tags.get("waterway") in WATER_CLASSES:
            kind, feature_class = "water", tags.get("waterway")
        if kind is None or feature_class is None:
            return

        geometry: list[tuple[float, float]] = []
        south, west, north, east = self.city["bbox"]
        for node in way.nodes:
            if (
                node.location.valid()
                and south <= node.location.lat <= north
                and west <= node.location.lon <= east
            ):
                geometry.append(self.project(node.location.lat, node.location.lon))
        if len(geometry) < 2:
            return

        feature: dict[str, Any] = {
            "kind": kind,
            "class": feature_class,
            "points": simplify(geometry),
        }
        if tags.get("name"):
            feature["name"] = tags.get("name")
        self.features.append(feature)

def download_extract(city: dict[str, Any]) -> Path:
    CACHE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    slug = city["bbbike"]
    target = CACHE_DIRECTORY / f"{slug}.osm.pbf"
    if target.exists() and target.stat().st_size > 0:
        return target
    source = f"https://download.bbbike.org/osm/bbbike/{slug}/{slug}.osm.pbf"
    temporary = target.with_suffix(".tmp")
    print(f"Downloading {city['city']}, {city['country']} from BBBike ...", flush=True)
    with requests.get(
        source,
        headers={"User-Agent": USER_AGENT},
        stream=True,
        timeout=(30, 180),
    ) as response:
        response.raise_for_status()
        with temporary.open("wb") as output:
            for chunk in response.iter_content(chunk_size=1024 * 1024):
                output.write(chunk)
    temporary.replace(target)
    return target


def create_plan(city: dict[str, Any]) -> dict[str, Any]:
    extract_path = download_extract(city)
    handler = CityPlanHandler(city)
    handler.apply_file(str(extract_path), locations=True)
    south, west, north, east = city["bbox"]
    center_lat = (south + north) / 2
    center_lon = (west + east) / 2
    print(f"Imported {len(handler.features)} features for {city['city']}", flush=True)
    return {
        "id": city["id"],
        "city": city["city"],
        "country": city["country"],
        "label": f"{city['city']}, {city['country']}",
        "bbox": city["bbox"],
        "source_url": (
            f"https://www.openstreetmap.org/#map={city['zoom']}/"
            f"{center_lat:.5f}/{center_lon:.5f}"
        ),
        "extract_url": (
            f"https://download.bbbike.org/osm/bbbike/{city['bbbike']}/"
            f"{city['bbbike']}.osm.pbf"
        ),
        "features": handler.features,
    }


def main() -> None:
    plans = [create_plan(city) for city in CITIES]
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "attribution": "Map data © OpenStreetMap contributors",
        "extract_provider": "BBBike.org",
        "license": "Open Database License (ODbL) 1.0",
        "license_url": "https://www.openstreetmap.org/copyright",
        "plans": plans,
    }
    OUTPUT_PATH.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"Wrote {OUTPUT_PATH.relative_to(REPOSITORY_ROOT)}", flush=True)


if __name__ == "__main__":
    main()
