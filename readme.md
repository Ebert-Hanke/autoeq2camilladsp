# AutoEq2CamillaDSP

This is a simple tool to easily create a configuration file for your Headphones or IEMs using Headphone-Correction-Data from Jaakko Pasanen's [AutoEq](https://github.com/jaakkopasanen/AutoEq) to use with Henrik Enquist's [CamillaDSP](https://github.com/HEnquist/camilladsp) which e.g is now available in [moOde audio player](https://github.com/moode-player/moode).

## Things ToDo

- [x] Better filtering of results (atm quite quick and dirty scraping)
- [x] Option to include individual `devices` section as `.yml` file (atm a standard section is included which you need to change)
- [ ] Option to include crossfeed for headphones based on different implementations
- [ ] Option to include highshelf and/or lowshelf with sensible defaults

## Devices Section

The CamillaDSP configuration starts with a `devices` section which will be specific to the equipment you are using. In order to include this section just put it in a `.yml` file and it can be read and added to your configuration.
