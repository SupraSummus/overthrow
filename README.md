## What?

Overthrow is a hobby proejct. It's goal is to create strategy game that doesn't have common flaws.
 * No positive feedback loop. The more you grow the harder it gets.
 * No races. Timing of your actions is not so important.
 * No action-per-minute madness. Clicking fast doesn't help much.

It has positive properties:
 * Simple rules. One type of unit, one type of order you can give, no terrain.
 * Emergent gameplay. Ingame tasks range from micro-management to long-term global strategies.
 * Rich social interactions. Politics are core for cooperation.
 * Little randomness.

## How?

Overthrow mechanics are simple:
 * Turns are "parallel" - every player give orders, then all of them are executed at the same time.
 * Map is composed of hexagonal tiles.
 * Each map tile produces set amount of army each turn for its owner.
 * Players can command armies to move across the map.
 * Armies fight with each other dealing proportional amount of damage each turn.
 * Players are limited in amount of armies they can move each turn. This forces them to cooperate.
 * Players can join "corporations" with tree-like management structure.
 * Players can leave and rearrange their coreporation's structure.
 * Armies and tiles can be transfered between players in single corporation. This consumes players' "actions per turn" capital.

## Installation

    poetry install
    poetry shell

    cp .env.example .env
    # adjust settings in .env

    ./manage.py migrate

    ./manage.py runserver
