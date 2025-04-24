# Pool Party

This code programs a
[XAIO ESP32C6 IoT Mini Development Board](https://www.amazon.com/dp/B0DDQ4WBKJ)
to control a relay for a pool heater.

You can get more information about the chip itself
[from the Seeed Studio website](https://wiki.seeedstudio.com/xiao_esp32c6_getting_started/).

# Parts

## Existing Valve

The valve to control the heater was a
[CVA-24 Valve Actuator](https://www.amazon.com/Yoursme-Actuator-Compatible-Replacement-Rotation/dp/B0D83HH9ZF)
which runs off 24V and expects to see a SPDT relay to decide whether to open or close.

## Power supply

120V AC power was available at the external location, so I added a
[simple transformer](https://www.amazon.com/dp/B0DRHPQ26G) to
go from 120V to a nominal 24V, and then a
[DC step down adjustable voltage regulator](https://www.amazon.com/gp/product/B0BB8YWBHX)
to power the controller when USB power is not present. The voltage adjustment
screw lets me run the voltage just slightly above the 3.3V setting to improve
the temperature sensors reliability.

## Temperature Sensors

The two [DS18S20 temperature sensors](https://www.amazon.com/dp/B0BPFYQT8C)
are used to read the temperature is of the pool water as well
as the roof temperature.

## Relay

I got a bunch of these
[3V relays](https://www.amazon.com/gp/product/B08W3XDNGK),
but only need one for this project.

## Case

All of these parts seem to fit into
[this watertight case](https://www.amazon.com/gp/product/B0B87V7QTH).

# Wiring

This is a very simple circuit. The pins used on the ESP32C6 are as
follows:

## OneWire Bus

D0 and D1 (GPIO0 and GPIO1) are used with pullup resistors to run the two
Dallas OneWire circuits. These could have been combined into one but this
gives some extra redundancy.

These pins require pullup resistors, which are supplied in the kit. The
sensor with the longer wire will require an additional pullup resistor if
the distance is very far.

## Relay output

D2 (GPIO2) is used to control the relay. When this pin is pulled high, the
relay is energized, which enables the water heater.

## LED output

No idea what exactly to do with this, probably want to send different flash
sequences based on how hot the water is or something.

# Code Summary

The code provides a web service that generates hand-crafted JSON responses
based on the results of all the temperature sensors. It connects to the
first seen known SSID from a list of SSIDs stored in
[src/secrets.rs](src/secrets.rs.example), which must be set up from the
example before you build it.

## Building

To run this code, once everything is set up you can just run `cargo run`.
This builds everything, flashes the chip, and starts the monitor so you
can see the debugging information. An example of the success case looks
like this:

```
I (533) main_task: Started on CPU0
I (533) main_task: Calling app_main()
I (533) erik: Starting up
I (553) erik: Setting up wifi
I (573) erik: configuring wifi
I (733) erik: scanning for APs
I (5833) erik: Found known ssid 'JaneO'
I (5833) erik: Connecting to ssid 'JaneO'
W (8563) wifi:(trc)band:2G, phymode:7
I (8593) erik: Setting up the temperat, highestRateIdx:0, lowestRateIdx:13, dataSchedTableSize:16
I (8593) erik: connectedure sensor
I (8593) erik: Temperature is 82.399994F
I (8593) erik: Setting up a web server
W (8603) wifi:<ba-add>idx:0, ifx:0, tid:1, TAHI:0x100d0d5, TALO:0x56259ce8, (ssn:1, win:64, cur_ssn:1), CONF:0xc0001005
I (8613) erik: Setting up the temperature probe
W (8613) erik: extern temperature probe reset failed on pin 10
I (8623) erik: flashing the user LED
```

At this point, you can access the device's webpage, which gives you
some json like this:

```
{ "units": "farenheit", "sensors": { "internal": "71.6" } }
```
