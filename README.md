# lucre.io

A fun little io game inspired by the tank io game genre (i.e. Diep.io and Arras.io).

This was just a personal project of mine I spent a lot of time working on. I originally started this just as a project for experience and to post on my GitHub, and that is what it is for now--though at one point I considered actually developing this and turning it into a real game. That said while I would've loved to do this I have many other projects and other things that I need done, so I just don't really have the time for this. As it is just completing what this game currently is took me nearly two months.

Anyways, this game uses my own maze generation system (though currently it just defaults to an open map, and may have unintended bugs with maze maps, especially with event things) and physics engine. 
Links to these projects below ->
https://github.com/Lucrehulk/2d-rust-collision-engine
https://github.com/Lucrehulk/Rust-Maze-Generator

It uses pretty standard tank classes and bosses, mainly based off diep.io but also has some things inspired by arras.io (e.g. Celestial bosses, Hexagons).

The color palette currently directly rips the arras.io and diep.io color schemes, with the base 4 team colors using the diep.io theme, and the rest of the colors using the arras.io default theme palette.

The only available mode is TDM (it will randomly pick the base structure and amount of teams [2 or 4]).

The game features an event system, which includes fun little minigames and events that are meant to spice up the boring TDM experience. I won't describe them all but I'll point out my top 5 (in no particular order) to provide an idea of what these are:
1. Egg Hunt - A big egg entity will spawn in the middle of the map. The goal is for teams to push the egg to their base. When this is completed, a boss will spawn on that team. The event ends when that boss is killed.
2. Bounty - A big yellow circle entity called the "Bounty" will spawn in the middle of the map. A player who touches it can claim the bounty, and will become a yellow color (ID 3), and will recieve maxed stats. Their score will also be set to something high like one million or something (I forget the exact and I'm too lazy to check). The event ends once the target is killed.
3. Celestial Battle - Celestial bosses will spawn on each team. The event is simple: whichever team's Celestial is standing last is the team that wins. Upon event completion everyone on the winning team will recieve 50k score.
4. Meteor Shower - Meteor entities will spawn--entities that will roam around the map in a straight and constant direction. They have an insanely high mass and can pretty much only be redirected by walls or hitting one another. Anyone who touches a meteor will take damage. After a certain amount of time all meteors will despawn.
5. Paint Job - Bases are all removed. Now, tiles that players move to become that player's color base tile. Teams must compete to have the most tiles by the end of the event duration. At the ends the event, the team with the most tiles on the map will be declared the winner and every player on it will recieve 50k score. After the event ends team bases are restored.

Also yes, I know the main.rs code is pretty shitty LOL. This was just a personal project of mine and not made to be great. The only thing that was heavily intended to be optimized in this was my own physics engine--which is a seperate project that is just used in this.

Some videos. May add more later.

[video (29).webm](https://github.com/user-attachments/assets/3e936409-d721-441e-833d-b6d2b95170c0)
[video (28).webm](https://github.com/user-attachments/assets/89081416-ce1d-4c97-bcb3-0e5ee1e087f4)
[video (27).webm](https://github.com/user-attachments/assets/a4430467-c3a8-47b3-ae68-91947c17a6ce)
