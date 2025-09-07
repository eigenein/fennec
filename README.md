<img src="logo.webp" alt="Logo" height="200">

# Fennec

[FoxESS plug-in home battery](https://www.nextenergy.nl/artikelen/voor-batterij-fanaten) steering based
on [NextEnergy real-time prices](https://www.nextenergy.nl/actuele-energieprijzen).

[![Build status](https://img.shields.io/github/actions/workflow/status/eigenein/fennec/check.yaml?style=for-the-badge)](https://github.com/eigenein/fennec/actions/workflows/check.yaml)
[![Activity](https://img.shields.io/github/commit-activity/y/eigenein/fennec?style=for-the-badge)](https://github.com/eigenein/fennec/commits/main/)

I built this because I wasn't happy with the out-of-the-box integration of FoxESS with NextEnergy. At the time of writing, NextEnergy offered two battery control strategies:

- **Self-consumption:** charges with excess PV power and discharges when PV power is insufficient. Unfortunately, this doesn't take advantage of dynamic prices at all.
- **Price steering:** charges when energy is cheap and discharges when it's expensive. However, it's not what I expected – it's just a «typical» daily schedule that doesn't adapt to different price patterns on different days, nor does it take advantage of excess PV power.

Fennec, on the other hand, uses extensive information to build an optimal charging schedule:

- Current battery charge
- Battery charging and discharging efficiency
- Energy price chart for upcoming hours
- Solar power forecast for upcoming hours
- Household standby power consumption
- Energy feed-in tariff («inkoopvergoeding»)

It optimizes two parameters: maximum charging rate and minimum discharge threshold to maximize profit.

Fennec is designed to run as a cron job, continuously refining and updating the schedule.

## Example Kubernetes job

```yaml
apiVersion: "batch/v1"
kind: "CronJob"
metadata:
  name: "fennec"
spec:
  timeZone: "Europe/Amsterdam"
  schedule: "*/30 * * * *"
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
          containers:
            - name: "fennec-job"
              image: "ghcr.io/eigenein/fennec:0.1.5"
              env:
              - name: "TZ"
                value: "Europe/Amsterdam"
              - name: "WEERLIVE_API_KEY"
                value: "..."
              - name: "FOX_ESS_SERIAL_NUMBER"
                value: "..."
              - name: "FOX_ESS_API_KEY"
                value: "..."
              - name: "LOGFIRE_TOKEN"
                value: "..."
              command:
                - "/fennec"
                - "hunt"
```

## Example

Given the current residual energy of 3.87 out of 8.44 kWh by 01:00, for the following price chart:

![Example price chart](example-prices.png)

and the following weather forecast:

![Example weather forecast](example-weather.png)

Fennec generates the following schedule:

```text
  INFO fennec::foxess::schedule Time slot start_time=01:00, end_time=02:00, working_mode=Charging
  INFO fennec::foxess::schedule Time slot start_time=02:00, end_time=06:00, working_mode=Balancing
  INFO fennec::foxess::schedule Time slot start_time=06:00, end_time=10:00, working_mode=Discharging
  INFO fennec::foxess::schedule Time slot start_time=10:00, end_time=11:00, working_mode=Balancing
  INFO fennec::foxess::schedule Time slot start_time=11:00, end_time=16:00, working_mode=Charging
  INFO fennec::foxess::schedule Time slot start_time=16:00, end_time=23:59, working_mode=Discharging
```

## Energy costs cheatsheet

![Price build-up](energy-costs.png)
