# AutoEq2CamillaDSP

This is a simple CLI tool to easily create a configuration file for your Headphones or IEMs using Headphone-Correction-Data from Jaakko Pasanen's [AutoEq](https://github.com/jaakkopasanen/AutoEq) to use with Henrik Enquist's [CamillaDSP](https://github.com/HEnquist/camilladsp). `CamillaDSP` is e.g included in [moOde audio player](https://github.com/moode-player/moode).  
Using it stand alone on all major systems is well documented in [Processing audio](https://github.com/HEnquist/camilladsp#processing-audio) of the documentation.

## Interactive Mode 
If you start the tool with no arguments from the command line with `autoeq2camilladsp` you will enter interactive mode which will lead you through the progress of creating your configuration.

## Non-Interactive Mode
To use the tool in other contexts e.g. a music player like `moOde`, there are some commands which will enable you to do so.  
If you start the tool with arguments, it will be run in non-interactive mode. To get a list of all available commands and options run `autoeq2camilladsp -h`.

### Available Commands
#### init
This will get the full list of entries from the AutoEq repository and output it to the terminal as JSON. It also includes a list of all the available presets.

``` json
{
"autoeqList": [
    {
      "name": "onkyo ie-fc300",
      "link": "/jaakkopasanen/AutoEq/blob/master/results/referenceaudioanalyzer/referenceaudioanalyzer_siec_harman_in-ear_2019v2/Onkyo%20IE-FC300"
    },
    {
      "name": "tozo nc7",
      "link": "/jaakkopasanen/AutoEq/blob/master/results/rtings/rtings_harman_in-ear_2019v2/TOZO%20NC7"
    },
    // ...
],
"crossfeedPresets" [
    "None",
    "PowChuMoy",
    "Mpm",
    "Natural"
]
}
```

## Devices Section
The CamillaDSP configuration starts with a `devices` section which will be specific to the equipment you are using. In order to include this section just put it in a `.yml` file and it can be read and added to your configuration.  
Please refer to the [CamillaDSP Readme](https://github.com/HEnquist/camilladsp#configuration) for more information about this section.
If you do not include your own `devices` file, the configuration will be built with a default which works as is in `moOde`.

## Crossfeed
You can include [Crossfeed](https://en.wikipedia.org/wiki/Crossfeed) in your configuration file.  
The basic principle of this is to reduce the channel separation of the stereo signal by feeding a little amount of the lower frequency range from left to right and vice versa.  

### Basic Crossfeed
```mermaid
graph LR
A[Left IN]--> G[Attenuation / Filters]--> B[Left OUT]
A --> E[Attenuation / Filters] --> D
C[Right IN]-->H[Attenuation / Filters]-->D[Right OUT]
C --> F[Attenuation / Filters] -->B
```

### General Information
Two my knowledge there are two main publications most implementation of crossfeed are based on:  
- *Stereophonic Earphones and Binaural Loudspeakers* by [Benjamin B. Bauer](https://en.wikipedia.org/wiki/Benjamin_Bauer) published JAES Volume 9 Number 2 in 1961
- [Improved Headphone Listening](https://www.linkwitzlab.com/headphone-xfeed.htm) by [Siegfried Linkwitz](https://en.wikipedia.org/wiki/Siegfried_Linkwitz) published in Audio in 1973  

The most widely used DSP implementation of crossfeed might be Boris Mikhaylov's [Bauer stereophonic-to-binuaral DSP / bs2b](http://bs2b.sourceforge.net) or `bs2b`. Mikhaylov also provides a lot of interesting background, research and explanation on his design decissions for `bs2b`.

Two `CamillaDSP` related crossfeed projects worth looking into are Yue Wang's [camilladsp-crossfeed](https://github.com/Wang-Yue/camilladsp-crossfeed) and [CamillaDSP-Monitor](https://github.com/Wang-Yue/CamillaDSP-Monitor).

I can also highly recommend Mikhail Naganov's [Electronic Projects](https://melp242.blogspot.com/) blog which provides a lot of in depth information on various audio topics.

### Pow Chu Moy Crossfeed
```mermaid
graph LR
A[Left IN]-- -2 dB --> G[Highshelf Filter 950 Hz +2 dB]--> B[Left OUT]
A -- -6 dB --> E[Lowpass Filter 700 Hz] --> D
C[Right IN]-- -2 db -->H[Highshelf Filter 950 Hz +2 dB]-->D[Right OUT]
C -- -6 dB --> F[Lowpass Filter 700 Hz] --> B
```

This preset is based on the analogue implementation by [Pow Chu Moy](https://jourshifi.wordpress.com/2016/03/17/the-hero-of-diy-audio-pow-chu-moy/) who designed [An Acoustic Simulator For Headphone Amplifiers](https://headwizememorial.wordpress.com/2018/03/09/an-acoustic-simulator-for-headphone-amplifiers/) which in turn is based on the implementation by [Siegfried Linkwitz](https://en.wikipedia.org/wiki/Siegfried_Linkwitz) which was published as [Improved Headphone Listening](https://www.linkwitzlab.com/headphone-xfeed.htm) 1973 in Audio.  

The DSP version of this draws from Boris Mikhaylov's [Bauer stereophonic-to-binuaral DSP / bs2b](http://bs2b.sourceforge.net) implementation in the widely used `bs2b`.  

### MPM Crossfeed
```mermaid
graph LR
A[Left IN]-- -2.3 dB --> G[Highshelf Filter 200 Hz +2.3 dB]--> B[Left OUT]
A -- -9.9 dB --> E[Highshelf Filter 750 Hz -0.3 dB] --> I[Peaking EQ 180 Hz +0.5 dB Q 0.55] --> D 
C[Right IN]-- -2.3 db -->H[Highshelf Filter 200 Hz +2.3 dB]-->D[Right OUT]
C -- -9.9 dB --> F[Highshelf Filter 750 Hz -0.3 dB] --> J[Peaking EQ 180 Hz +0.5 dB Q 0.55] --> B
```

This preset is based on research and "reverse engineering" done by Mikhail Naganov and published on his blog [Electronic Projects](https://melp242.blogspot.com/) in [Reconstructing SPL Phonitor Mini Crossfeed with DSP](https://melp242.blogspot.com/2017/01/reconstructing-spl-phonitor-mini.html) in 2017.  
I reached out to Mikhail who was so kind to contribute this implementation and also provide a lot of insight to my hobbyist research on the crossfeed topic in general. Thanks! 

### Natural Crossfeed

*work in progress*
```mermaid
graph LR
A[Left IN]-- -1.5 dB --> G[Highshelf Filter 900 Hz +1.5 dB]--> B[Left OUT]
A -- -9.5 dB --> E[Lowpass Filter 650 Hz] --> D
C[Right IN]-- -1.5 db -->H[Highshelf Filter 900 Hz +1.5 dB]-->D[Right OUT]
C -- -9.5 dB --> F[Lowpass Filter 650 Hz] --> B
```



