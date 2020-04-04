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

World setting
 * Map is composed of hexagonal tiles.
 * Each tile has owner (player).
 * Each tile has some armies on it (sometimes 0. Army count is capped to max CP count that players can accumulate.
 * Each map tile increases it's resources each turn. Growth is logarithmic (the more resources, the slower the growth). Resources are "potential for armies", and can be converted into armies by players.

Battle mechanics
 * Turns are "parallel" - every player give orders, then all of them are executed at the same time.
 * Players are limited in "command points" (CPs) they can expend each turn.
 * Unused "command points" accumulate up to set cap. This ensures one can be idle for limited amount of time without much loss.
 * Players can convert tile resources to armies. This costs them proportional amount of "command points".
 * Players can command armies to move across the map. Movement of each army by one tile costs the army owner one CP.
 * Armies of opposing sides fight with each other dealing proportional amount of damage each turn. Fighting with enemy costs one CP for each attacking army.

Cooperation mechanics
 * Player can join "corporations" with tree-like management structure.
 * Player can detach from corportation as she wish, taking whole subtree with her.
 * Player can rearrange their subtree (including the player) of corporation's structure.
 * Armies and tiles can be transfered between players in single corporation, up or down management structure. No non-tree transfers. This consumes players' CP capital.

## Installation

    poetry install
    poetry shell

    cp .env.example .env
    # adjust settings in .env

    ./manage.py migrate

    ./manage.py runserver
