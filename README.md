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
- Battery parasitic load
- Energy price chart for upcoming hours
- Average household standby power consumption from Home Assistant
- Energy feed-in tariff («inkoopvergoeding»)

Fennec is designed to run as a cron job, continuously refining and updating the schedule.

## Usage

### Running as a Kubernetes job

> [!IMPORTANT]
> At the moment, I only have `aarch64-unknown-linux-gnu` Docker builds.

```yaml
apiVersion: "batch/v1"
kind: "CronJob"
metadata:
  name: "fennec"
spec:
  timeZone: "Europe/Amsterdam"
  schedule: "1,31 * * * *"
  startingDeadlineSeconds: 900
  concurrencyPolicy: "Replace"
  successfulJobsHistoryLimit: 1
  jobTemplate:
    spec:
      backoffLimit: 3
      ttlSecondsAfterFinished: 86400
      template:
        spec:
          restartPolicy: "OnFailure"
          containers:
            - name: "fennec-job"
              image: "ghcr.io/eigenein/fennec:0.33.0"
              env:
                - name: "TZ"
                  value: "Europe/Amsterdam"
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
                - name: "HOME_ASSISTANT_API_BASE_URL"
                  value: "..."
                - name: "HOME_ASSISTANT_TOTAL_USAGE_ENTITY_ID"
                  value: "sensor.custom_fennec_hourly_total_energy_usage"
                - name: "HOME_ASSISTANT_TOTAL_SOLAR_YIELD_ENTITY_ID"
                  value: "sensor.custom_fennec_hourly_total_solar_yield"
              command:
                - "/fennec"
                - "hunt"
```

## Home Assistant integration

### Example

```yaml
template:
  - triggers:
      - trigger: "time_pattern"
        minutes: "/5"
    sensor:
      - name: "Fennec total solar yield"
        unit_of_measurement: "kWh"
        unique_id: "custom_fennec_hourly_total_solar_yield"
        default_entity_id: "sensor.custom_fennec_hourly_total_solar_yield"
        icon: "mdi:flash"
        state_class: "total"
        state: "{{ states('sensor.sb2_5_1vl_40_555_total_yield') }}"
        attributes:
          custom_now: "{{ now() }}" # force update
      - name: "Fennec total energy usage"
        unit_of_measurement: "kWh"
        unique_id: "custom_fennec_hourly_total_energy_usage"
        default_entity_id: "sensor.custom_fennec_hourly_total_energy_usage"
        icon: "mdi:flash"
        state_class: "total"
        state: |
          {{
              states('sensor.p1_meter_energy_import') | float
            + states('sensor.sb2_5_1vl_40_555_total_yield') | float
            + states('sensor.battery_socket_energy_export') | float
            - states('sensor.p1_meter_energy_export') | float
            - states('sensor.battery_socket_energy_import') | float
          }}
        attributes:
          custom_now: "{{ now() }}" # force update
```
