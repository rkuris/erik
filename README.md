# Pool Party

This code programs a [XAIO ESP32C6 IoT Mini Development Board]
(https://www.amazon.com/dp/B0DDQ4WBKJ)
to control a relay for a pool heater.

You can get more information about the chip itself
[from the Seeed Studio website](https://wiki.seeedstudio.com/xiao_esp32c6_getting_started/)

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
sequences based on how hot the water is or something
