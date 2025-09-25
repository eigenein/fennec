<img src="logo.webp" alt="Logo" height="200">

# Fennec

[FoxESS plug-in home battery](https://www.nextenergy.nl/artikelen/voor-batterij-fanaten) steering based
on [NextEnergy real-time prices](https://www.nextenergy.nl/actuele-energieprijzen).

[![Build status](https://img.shields.io/github/actions/workflow/status/eigenein/fennec/check.yaml?style=for-the-badge)](https://github.com/eigenein/fennec/actions/workflows/check.yaml)
[![Codecov](https://img.shields.io/codecov/c/github/eigenein/fennec?style=for-the-badge)](https://app.codecov.io/gh/eigenein/fennec)
[![Activity](https://img.shields.io/github/commit-activity/y/eigenein/fennec?style=for-the-badge)](https://github.com/eigenein/fennec/commits/main/)

I built this because I wasn't happy with the out-of-the-box integration of FoxESS with NextEnergy. At the time of writing, NextEnergy offered two battery control strategies:

- **Self-consumption:** charges with excess PV power and discharges when PV power is insufficient. Unfortunately, this doesn't take advantage of dynamic prices at all.
- **Price steering:** charges when energy is cheap and discharges when it's expensive. However, it's not what I expected – it's just a «typical» daily schedule that doesn't adapt to different price patterns on different days, nor does it take advantage of excess PV power.

Fennec, on the other hand, uses extensive information to build an optimal charging schedule:

- Current battery charge
- Battery charging and discharging efficiency
- Energy price chart for upcoming hours
- Solar power forecast for upcoming hours
- Average household standby power consumption from Home Assistant
- Energy feed-in tariff («inkoopvergoeding»)

Fennec is designed to run as a cron job, continuously refining and updating the schedule.

## Example prognosis

```text
╭───────┬────────────┬──────────┬──────────┬─────────────┬──────────┬──────────┬────────────┬─────────╮
│ Time  ┆ Grid rate  ┆ Stand-by ┆ Solar    ┆ Mode        ┆ Before   ┆ After    ┆ Grid usage ┆ Loss    │
╞═══════╪════════════╪══════════╪══════════╪═════════════╪══════════╪══════════╪════════════╪═════════╡
│ 23:00 ┆ 0.23 €/kWh ┆ 0.55 kW  ┆   0 W/m² ┆ Idle        ┆ 2.48 kWh ┆ 2.46 kWh ┆ +0.55 kWh  ┆ +0.13 € │
│ 00:00 ┆ 0.23 €/kWh ┆ 0.48 kW  ┆   0 W/m² ┆ Idle        ┆ 2.46 kWh ┆ 2.44 kWh ┆ +0.48 kWh  ┆ +0.11 € │
│ 01:00 ┆ 0.23 €/kWh ┆ 0.39 kW  ┆   0 W/m² ┆ Idle        ┆ 2.44 kWh ┆ 2.42 kWh ┆ +0.39 kWh  ┆ +0.09 € │
│ 02:00 ┆ 0.23 €/kWh ┆ 0.36 kW  ┆   0 W/m² ┆ Idle        ┆ 2.42 kWh ┆ 2.40 kWh ┆ +0.36 kWh  ┆ +0.08 € │
│ 03:00 ┆ 0.23 €/kWh ┆ 0.33 kW  ┆   0 W/m² ┆ Idle        ┆ 2.40 kWh ┆ 2.38 kWh ┆ +0.33 kWh  ┆ +0.08 € │
│ 04:00 ┆ 0.24 €/kWh ┆ 0.34 kW  ┆   0 W/m² ┆ Idle        ┆ 2.38 kWh ┆ 2.36 kWh ┆ +0.34 kWh  ┆ +0.08 € │
│ 05:00 ┆ 0.24 €/kWh ┆ 0.35 kW  ┆   0 W/m² ┆ Idle        ┆ 2.36 kWh ┆ 2.34 kWh ┆ +0.35 kWh  ┆ +0.08 € │
│ 06:00 ┆ 0.26 €/kWh ┆ 0.33 kW  ┆   0 W/m² ┆ Balancing   ┆ 2.34 kWh ┆ 1.99 kWh ┆ +0.00 kWh  ┆ +0.00 € │
│ 07:00 ┆ 0.28 €/kWh ┆ 0.34 kW  ┆   6 W/m² ┆ Balancing   ┆ 1.99 kWh ┆ 1.64 kWh ┆ -0.00 kWh  ┆ -0.00 € │
│ 08:00 ┆ 0.30 €/kWh ┆ 0.49 kW  ┆  75 W/m² ┆ Discharging ┆ 1.63 kWh ┆ 0.84 kWh ┆ -0.43 kWh  ┆ -0.12 € │
│ 09:00 ┆ 0.27 €/kWh ┆ 0.56 kW  ┆ 199 W/m² ┆ Idle        ┆ 0.84 kWh ┆ 0.82 kWh ┆ +0.07 kWh  ┆ +0.02 € │
│ 10:00 ┆ 0.24 €/kWh ┆ 0.61 kW  ┆ 330 W/m² ┆ Idle        ┆ 0.82 kWh ┆ 0.80 kWh ┆ -0.20 kWh  ┆ -0.04 € │
│ 11:00 ┆ 0.16 €/kWh ┆ 0.88 kW  ┆ 413 W/m² ┆ Charging    ┆ 0.80 kWh ┆ 1.93 kWh ┆ +1.07 kWh  ┆ +0.17 € │
│ 12:00 ┆ 0.14 €/kWh ┆ 1.05 kW  ┆ 443 W/m² ┆ Charging    ┆ 1.92 kWh ┆ 3.05 kWh ┆ +1.16 kWh  ┆ +0.16 € │
│ 13:00 ┆ 0.16 €/kWh ┆ 1.06 kW  ┆ 496 W/m² ┆ Charging    ┆ 3.04 kWh ┆ 4.17 kWh ┆ +1.04 kWh  ┆ +0.17 € │
│ 14:00 ┆ 0.16 €/kWh ┆ 0.93 kW  ┆ 504 W/m² ┆ Charging    ┆ 4.16 kWh ┆ 5.29 kWh ┆ +0.90 kWh  ┆ +0.14 € │
│ 15:00 ┆ 0.19 €/kWh ┆ 1.35 kW  ┆ 432 W/m² ┆ Charging    ┆ 5.28 kWh ┆ 6.41 kWh ┆ +1.49 kWh  ┆ +0.28 € │
│ 16:00 ┆ 0.23 €/kWh ┆ 1.09 kW  ┆ 332 W/m² ┆ Balancing   ┆ 6.40 kWh ┆ 6.11 kWh ┆ -0.00 kWh  ┆ -0.00 € │
│ 17:00 ┆ 0.25 €/kWh ┆ 0.83 kW  ┆ 219 W/m² ┆ Balancing   ┆ 6.10 kWh ┆ 5.79 kWh ┆ -0.00 kWh  ┆ -0.00 € │
│ 18:00 ┆ 0.28 €/kWh ┆ 0.89 kW  ┆  94 W/m² ┆ Discharging ┆ 5.78 kWh ┆ 4.93 kWh ┆ -0.14 kWh  ┆ -0.04 € │
│ 19:00 ┆ 0.29 €/kWh ┆ 0.77 kW  ┆   8 W/m² ┆ Discharging ┆ 4.92 kWh ┆ 4.07 kWh ┆ -0.05 kWh  ┆ -0.01 € │
│ 20:00 ┆ 0.30 €/kWh ┆ 0.75 kW  ┆   0 W/m² ┆ Discharging ┆ 4.06 kWh ┆ 3.21 kWh ┆ -0.05 kWh  ┆ -0.01 € │
│ 21:00 ┆ 0.27 €/kWh ┆ 0.99 kW  ┆   0 W/m² ┆ Discharging ┆ 3.20 kWh ┆ 2.35 kWh ┆ +0.19 kWh  ┆ +0.05 € │
│ 22:00 ┆ 0.26 €/kWh ┆ 0.60 kW  ┆   0 W/m² ┆ Discharging ┆ 2.34 kWh ┆ 1.49 kWh ┆ -0.20 kWh  ┆ -0.05 € │
│ 23:00 ┆ 0.24 €/kWh ┆ 0.55 kW  ┆ unknown  ┆ Discharging ┆ 1.48 kWh ┆ 0.84 kWh ┆ -0.04 kWh  ┆ -0.01 € │
╰───────┴────────────┴──────────┴──────────┴─────────────┴──────────┴──────────┴────────────┴─────────╯
```

## Caveats

- [FoxESS Cloud](https://www.foxesscloud.com/public/i18n/en/OpenApiDocument.html#set20the20time20segment20information0a3ca20id3dset20the20time20segment20information7193e203ca3e) only allows up to 8 schedule slots. It might happen that Fennec would build a schedule with more slots. In that case, only the upcoming 8 slots will be pushed. So, make sure to run Fennec periodically.

## Example Kubernetes job

```yaml
apiVersion: "batch/v1"
kind: "CronJob"
metadata:
  name: "fennec"
spec:
  timeZone: "Europe/Amsterdam"
  schedule: "5 * * * *"
  startingDeadlineSeconds: 600
  concurrencyPolicy: "Replace"
  successfulJobsHistoryLimit: 1
  jobTemplate:
    spec:
      backoffLimit: 3
      ttlSecondsAfterFinished: 86400
      template:
        spec:
          restartPolicy: "OnFailure"
          volumes:
            - name: "cache"
              hostPath:
                path: "/Users/eigenein/.fennec/cache.toml"
                type: "FileOrCreate"
          containers:
            - name: "fennec-job"
              image: "ghcr.io/eigenein/fennec:0.17.2"
              volumeMounts:
                - mountPath: "/cache.toml"
                  name: "cache"
              env:
              - name: "TZ"
                value: "Europe/Amsterdam"
              - name: "PV_SURFACE_M2"
                value: "2.4"
              - name: "WEERLIVE_API_KEY"
                value: "..."
              - name: "FOX_ESS_SERIAL_NUMBER"
                value: "..."
              - name: "FOX_ESS_API_KEY"
                value: "..."
              - name: "LOGFIRE_TOKEN"
                value: "..."
              - name: "HEARTBEAT_URL"
                value: "https://uptime.betterstack.com/api/v1/heartbeat/..."
              - name: "HOME_ASSISTANT_ACCESS_TOKEN"
                value: "..."
              - name: "HOME_ASSISTANT_TOTAL_ENERGY_USAGE_URL"
                value: "https://.../api/states/sensor.custom_total_energy_usage"
              command:
                - "/fennec"
                - "hunt"
```
