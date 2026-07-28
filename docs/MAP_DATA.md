# City map data

The visual city uses compact, offline street geometry for three representative cities:

| Game map asset | City | Country | Imported features |
|---|---|---|---:|
| `osm:new-delhi-india` | New Delhi | India | 1,167 |
| `osm:beijing-china` | Beijing | China | 2,043 |
| `osm:rio-de-janeiro-brazil` | Rio de Janeiro | Brazil | 2,420 |

The source data is OpenStreetMap data distributed through the published BBBike city extracts. It is licensed under the Open Database License (ODbL) 1.0. The application displays “Map data © OpenStreetMap contributors” with links to the relevant OpenStreetMap view and copyright/license page.

Only major roads, rail lines and waterways inside a small central bounding box are retained. Coordinates are projected to the game’s 1000 × 600 logical canvas and simplified. The application does not download map tiles and has no runtime dependency on OpenStreetMap or BBBike.

These maps provide geographic texture only. Scenarios, events, characters, civic institutions and their marker positions are fictional and do not describe the real city or its government.

## Rebuild the map data

From the repository root:

```powershell
python -m pip install -r scripts/requirements-map-import.txt
python scripts/import_city_plans.py
```

The importer caches the source `.osm.pbf` extracts under `.cache/city-plans/` and writes the tracked, normalized asset to `web/city-data/city-plans.json`. Review the bounding boxes and city list in `scripts/import_city_plans.py` before regenerating.

After regeneration, rebuild the frontend and standalone file:

```powershell
Set-Location web
npm run typecheck
npm test
npm run build
npm run standalone
```

## Sources and license

- OpenStreetMap copyright and attribution: <https://www.openstreetmap.org/copyright>
- BBBike published city extracts: <https://download.bbbike.org/osm/bbbike/>
- New Delhi extract: <https://download.bbbike.org/osm/bbbike/NewDelhi/>
- Beijing extract: <https://download.bbbike.org/osm/bbbike/Beijing/>
- Rio de Janeiro extract: <https://download.bbbike.org/osm/bbbike/RiodeJaneiro/>
