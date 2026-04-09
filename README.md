<img src="logo.webp" alt="Logo" height="200">

# Fennec

[![Build status](https://img.shields.io/github/actions/workflow/status/eigenein/fennec/check.yaml?style=for-the-badge)](https://github.com/eigenein/fennec/actions/workflows/check.yaml)
[![Codecov](https://img.shields.io/codecov/c/github/eigenein/fennec?style=for-the-badge)](https://app.codecov.io/gh/eigenein/fennec)
[![Activity](https://img.shields.io/github/commit-activity/y/eigenein/fennec?style=for-the-badge)](https://github.com/eigenein/fennec/commits/main/)

[FoxESS plug-in home battery](https://www.nextenergy.nl/artikelen/voor-batterij-fanaten) steering based
on [Next Energy](https://www.nextenergy.nl/actuele-energieprijzen) or [Frank Energie](https://www.frankenergie.nl/nl/dynamisch-energiecontract/dynamische-energieprijzen) real-time rates and:

- Current battery charge
- Charging and discharging efficiency
- Parasitic BMS load
- Average household consumption per price interval

## Acknowledgments

### Modbus Register Tables

- [openhab/openhab-addons](https://raw.githubusercontent.com/openhab/openhab-addons/refs/heads/main/bundles/org.openhab.binding.modbus.foxinverter/src/main/java/org/openhab/binding/modbus/foxinverter/internal/MQ2200InverterRegisters.java)
- [solakon-de/solakon-one-homeassistant](https://raw.githubusercontent.com/solakon-de/solakon-one-homeassistant/refs/heads/main/custom_components/solakon_one/const.py)
- [wimb0/home-assistant-nextenergy-battery-modbus](https://raw.githubusercontent.com/wimb0/home-assistant-nextenergy-battery-modbus/refs/heads/main/custom_components/nextenergy_battery/const.py)
