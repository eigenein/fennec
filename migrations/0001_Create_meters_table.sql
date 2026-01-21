-- Migration number: 0001 	 2026-01-20T21:56:06.447Z
CREATE TABLE meters (
    timestamp                   INTEGER NOT NULL PRIMARY KEY,
    p1_import_kwh               REAL NOT NULL,
    p1_export_kwh               REAL NOT NULL,
    battery_import_kwh          REAL NOT NULL,
    battery_export_kwh          REAL NOT NULL,
    battery_residual_energy_kwh REAL NOT NULL
) WITHOUT ROWID, STRICT;
