#!/usr/bin/env python3
"""Generate Floor 4 rooms and update rooms.json and maps.json for Layla.
Floor 4 layout: 100 rooms (10x10 grid, larger than previous floors).
Zone 1: The Village (35 rooms: x 0..4, y 3..9)
Zone 2: The Mana Mine (35 rooms: x 5..9, y 3..9)
Zone 3: The Guard Camp (30 rooms: x 0..9, y 0..2)
"""

import json
import os
import sys

# Define all 100 rooms with their metadata
# Grid: (x, y) with x in 0..9, y in 0..9

ROOM_DATA = {}

def add_room(x, y, room_id, title, summary, inspect_text, feat_label, feat_aliases, feat_inspect):
    ROOM_DATA[(x, y)] = {
        "id": room_id,
        "x": x,
        "y": y,
        "title": title,
        "summary": summary,
        "inspect_text": inspect_text,
        "features": [
            {
                "id": f"{room_id}-feature",
                "label": feat_label,
                "aliases": feat_aliases,
                "inspect_text": feat_inspect,
            }
        ],
    }

# ==============================================================================
# ZONE 3: THE GUARD CAMP (y = 0..2, x = 0..9) - 30 rooms
# ==============================================================================

# Row 0
add_room(
    0, 0, "camp_nw_tower", "Northwest Watchtower",
    "A tall wooden platform looking out over the dark cave ceiling. Cold drafts blow down from cracks in the stone roof.",
    "Wooden braces hold up a sturdy catwalk twenty feet above the fort floor. Crossbow bolts lie ready in open wooden bins.",
    "the wooden catwalk", ["catwalk", "tower", "platform", "wood"],
    "Planks of heavy pine nailed across oak beams, looking down across the fortified camp."
)
add_room(
    1, 0, "camp_north_wall_w", "West Rampart Walk",
    "A high walkway behind sharpened log palisades. Sentry shields hang against the wooden rail.",
    "Torches set in iron hoops light the perimeter wall. Soldiers can patrol along this plank floor to spot trouble from afar.",
    "the sharpened palisade logs", ["palisade", "logs", "wall", "spikes"],
    "Thick tree trunks stripped of bark and carved to sharp points, lashed tight with thick hemp ropes."
)
add_room(
    2, 0, "camp_patrol_post", "North Patrol Post",
    "A sheltered alcove on the north wall where sentries rest between patrol shifts.",
    "A small charcoal brazier keeps the guard post warm. Scratched game boards are cut into the bench where off-duty soldiers play dice.",
    "the charcoal brazier", ["brazier", "charcoal", "heater", "fire"],
    "An iron dish glowing with red coals that keep off the chill of the north cavern."
)
add_room(
    3, 0, "camp_officers_tent", "Officer's Bunk",
    "A neat canvas tent with a cot and small footlocker for the camp lieutenants.",
    "Polished leather boots and a spare helmet sit on a wooden stool. A ledger tracks daily guard duty rotations.",
    "the officer's footlocker", ["footlocker", "locker", "chest", "cot"],
    "A brass-hinged wooden box holding clean wool socks, a guard roster, and a short iron knife."
)
add_room(
    4, 0, "camp_gate_platform", "Teleport Platform Approach",
    "A wide stone ramp leading up to the great magic archway. White runes carved along the curb pulse with pale light.",
    "Heavy flagstones lead directly toward the teleportation gate. The air smells sharp like lightning before a summer storm.",
    "the carved stone ramp", ["ramp", "stones", "path", "curb"],
    "Smooth gray stones edged with glowing runes that brighten as you get closer to the gate."
)
add_room(
    5, 0, "teleport_gate", "The Teleportation Gate",
    "A huge circular stone archway built against the back cave wall. Carved white runes glow with bright light and emit a deep electrical hum.",
    "This is the only way up to Floor 5. The white stone ring stands twenty feet high, crackling with raw magical energy. A stone pedestal in front of the gate has a circular slot for a teleportation spell.",
    "the glowing stone archway", ["gate", "archway", "stone ring", "arch", "runes", "portal"],
    "A massive ring of carved white marble. Ribbons of pale light swirl inside the opening, humming with power."
)
add_room(
    6, 0, "camp_priest_tent", "Priest's Pavilion",
    "A white silk pavilion decorated with silver eye symbols. Incense burns in bronze censers, smelling of dry flowers and wax.",
    "Priest Malas stays here while waiting to claim the village offering. Velvet pillows rest on a wool rug, surrounded by quiet candle stands.",
    "the silver eye banner", ["banner", "symbol", "eye", "censer"],
    "A silver flag embroidered with the unblinking eye of the temple priests, watching over the camp."
)
add_room(
    7, 0, "camp_north_wall_e", "East Rampart Walk",
    "An elevated wooden walk along the eastern ramparts. Below, you can see the top of the crystal mine pits.",
    "Guards patrol here day and night to ensure no miners try to sneak out with stolen mana crystals.",
    "the guard lookout ledge", ["lookout", "ledge", "rail", "planks"],
    "A high wooden railing where soldiers lean and watch the eastern mine tunnels below."
)
add_room(
    8, 0, "camp_signal_post", "Signal Fire Platform",
    "A stone platform built on the highest rock pillar. A stack of dry pitch-soaked logs waits ready for a beacon fire.",
    "If miners riot or beasts break in, this fire signals the guards on the upper floors. An iron gong hangs beside the wood pile.",
    "the signal gong", ["gong", "iron gong", "bell", "hammer"],
    "A heavy bronze disc struck with an iron hammer to sound camp-wide alarms."
)
add_room(
    9, 0, "camp_ne_tower", "Northeast Watchtower",
    "A sturdy timber tower looking down into the eastern mine chasms and cart rails.",
    "From up here, the whole mine looks like a web of blue and purple lights. Crossbow quivers hang from every corner post.",
    "the corner tower posts", ["posts", "tower", "beams", "timbers"],
    "Heavy pine pillars notched and pegged together to withstand cave tremors."
)

# Row 1
add_room(
    0, 1, "camp_west_wall", "West Stockade Wall",
    "A solid line of pointed pine logs backed by dirt mounds. Soldiers patrol the packed dirt walkway below.",
    "The wall separates the soldiers from the cold, dark western caves where no one goes.",
    "the packed dirt rampart", ["rampart", "dirt", "walkway", "mound"],
    "Firm earth packed between timber retaining walls, creating an elevated walkway."
)
add_room(
    1, 1, "camp_barracks_1", "West Soldier Barracks",
    "A long wooden building lined with double-deck bunk beds and straw mattresses. Off-duty soldiers sleep or play cards.",
    "Wool blankets are piled on the beds. The room smells of stale sweat, leather wax, and pipe smoke.",
    "the row of bunks", ["bunks", "beds", "mattresses", "straw"],
    "Simple pine bunk beds with rough wool blankets folded at the foot of each mattress."
)
add_room(
    2, 1, "camp_mess_hall", "Camp Mess Hall",
    "Long trestle tables and benches fill this wooden hall. Iron pots bubble over an open hearth.",
    "Soldiers eat hot mutton stew and fresh bread here. The room smells of roasted grease, garlic, and spilled beer.",
    "the big stew pot", ["pot", "stew", "hearth", "kettle"],
    "A three-legged iron cauldron bubbling with potatoes, salt meat, and root vegetables."
)
add_room(
    3, 1, "camp_armory", "Guard Armory",
    "Racks of polished iron spears, shields painted red, and rows of heavy crossbows line the stone walls.",
    "Armored suits stand on wooden mannequins. Iron weapon trunks line the floor, each secured with stout brass locks.",
    "the weapon racks", ["racks", "spears", "shields", "crossbows"],
    "Solid ash wood racks holding sharp iron spears and heavy military crossbows."
)
add_room(
    4, 1, "command_tent", "The Guard Command Tent",
    "A large military tent inside the camp stockade. An iron desk holds maps of the caves, and a heavy iron chest sits chained to the center tent pole.",
    "Red cloth banners hang from the tent frame. The room smells of lamp oil, roasted meat, and wet wool. In the corner, a loose floorboard covers the old drainage pipe. Chained to the center pole is a padlocked iron chest.",
    "the chained iron chest", ["chest", "iron chest", "box", "trunk"],
    "A heavy iron lockbox wrapped in chains. A keyhole on the front is shaped like a double circle. The Teleportation Scroll is locked inside."
)
add_room(
    5, 1, "camp_upper_courtyard", "Upper Camp Courtyard",
    "An open gravel yard between the command tent and the prison cage. Torches flicker in iron wall brackets.",
    "Patrols cross here frequently as they rotate between watch posts. The ground is packed hard by hundreds of iron-shod boots.",
    "the iron wall torches", ["torches", "brackets", "flames", "iron"],
    "Heavy iron sconces holding pitch-soaked torches that cast dancing yellow shadows across the gravel."
)
add_room(
    6, 1, "prison_cage", "The Prison Cage",
    "A courtyard behind high wooden walls. An iron cage hangs from heavy chains over a dry stone pit, guarded by torchlight.",
    "The cage is made of thick square iron bars. Inside, an eleven-year-old boy in dusty work clothes sits on the cold floor, glaring fiercely through the bars. Two guards stand nearby, keeping watch.",
    "the hanging iron cage", ["cage", "iron cage", "bars", "prison"],
    "A heavy iron cage suspended ten feet off the ground over a rocky pit. The door is locked with a heavy steel padlock."
)
add_room(
    7, 1, "camp_barracks_2", "East Soldier Barracks",
    "A second sleeping hall for the eastern garrison. Racks of dry boots sit near a warm iron stove.",
    "Footlockers are shoved under bunks. Shield coats and red cloaks hang from wooden pegs along the timber wall.",
    "the iron heating stove", ["stove", "heater", "pipes", "fire"],
    "A pot-bellied cast-iron stove that vents smoke up through a tin pipe into the high cavern ceiling."
)
add_room(
    8, 1, "camp_supply_store", "Camp Supply Store",
    "A large tent packed with sacks of grain, barrels of salted fish, coils of rope, and spare iron nails.",
    "Supplies for a three-month siege are stored here under guard. The air smells of salt, dried apples, and canvas.",
    "the grain sacks and barrels", ["sacks", "barrels", "grain", "supplies"],
    "Burlap sacks stamped with the royal crest, stacked neatly five high on wooden pallets."
)
add_room(
    9, 1, "camp_east_wall", "East Stockade Wall",
    "A high timber wall overlooking the rocky eastern ravine. Heavy support beams brace the log palisade.",
    "The cliff drops steeply on the other side. A small sentry booth keeps the rain of condensation off the guard on watch.",
    "the sentry shelter", ["shelter", "booth", "shack", "post"],
    "A tiny timber shack with a sloped shingle roof where a guard stands out of the damp."
)

# Row 2
add_room(
    0, 2, "camp_sw_tower", "Southwest Watchtower",
    "A timber watchtower looking south toward the village roofs and north into the soldiers' yard.",
    "A spiral ladder leads up to the watch platform. A brass horn hangs ready to call the garrison to arms.",
    "the brass horn", ["horn", "brass horn", "trumpet", "alarm"],
    "A curved brass war horn used to signal shift changes and sound the alarm."
)
add_room(
    1, 2, "camp_smithy", "Camp Smithy",
    "A smoky open-air workshop with a roaring stone furnace and a heavy iron anvil.",
    "The camp blacksmith pounds out glowing iron shoes for mules and straightens damaged spear tips. Sparks shower over the dirt floor.",
    "the heavy iron anvil", ["anvil", "forge", "hammer", "iron"],
    "A massive black anvil ringed with hammer marks, anchored firmly to a thick oak stump."
)
add_room(
    2, 2, "camp_archery_range", "Archery Practice Yard",
    "Straw target dummies stuffed with dried grass stand against a tall mound of dirt.",
    "Dozens of splintered crossbow bolts stick out from the wooden targets. Guards practice their aim here every morning.",
    "the straw targets", ["targets", "dummies", "straw", "bolts"],
    "Human-shaped targets woven from tough mountain straw, pocked with crossbow holes."
)
add_room(
    3, 2, "camp_training_yard", "Drill Ground",
    "A wide patch of packed sand where soldiers spar with wooden practice swords and shields.",
    "Deep footprints and scuff marks cover the arena. Racks of dulled wooden swords lean against the arena fence.",
    "the wooden practice swords", ["swords", "practice", "wood", "weapons"],
    "Heavy ash wood clubs shaped like short swords, weighted with lead inserts."
)
add_room(
    4, 2, "camp_gate", "The Guard Camp Gate",
    "A high wooden wall made of sharpened logs blocks the canyon road. Two guard towers look down over a heavy wooden gate barred with iron.",
    "Bright torches burn in iron baskets on the wooden catwalks. Armored soldiers pace the wall with loaded crossbows. The gate is firmly locked, and the guards shout down to warn everyone away.",
    "the heavy barred gate", ["gate", "barred gate", "wooden gate", "doors", "spikes"],
    "Thick wooden doors reinforced with black iron plates. A heavy iron bar holds them shut from the inside."
)
add_room(
    5, 2, "camp_lower_courtyard", "Lower Camp Courtyard",
    "The inner yard just behind the main gate. Wagons are unloaded here and inspection lines are formed.",
    "Wheel ruts groove the dirt. Guards check passes and inspect crates brought in from the village.",
    "the wagon wheel ruts", ["ruts", "ground", "dirt", "tracks"],
    "Deep wagon ruts pressed into the clay floor, dried hard by the warm cave air."
)
add_room(
    6, 2, "camp_guardhouse", "Sentry Guardhouse",
    "A low stone building where gate guards stay on duty. A desk holds the village passbook and iron keys.",
    "A small iron gate opens into a holding cell where rowdy miners spend the night before being kicked back out.",
    "the gate passbook", ["passbook", "book", "desk", "ledger"],
    "A thick leather book recording every cart of ore and visitor entering the camp."
)
add_room(
    7, 2, "camp_lumber_yard", "Palisade Timber Yard",
    "Stacks of fresh pine logs and cedar poles used to repair the fort walls and mine shoring.",
    "The air smells of pine resin and saw chips. Two-man crosscut saws hang on wooden pegs under a tarp.",
    "the stack of pine logs", ["logs", "timber", "wood", "pile"],
    "Straight pine trunks seasoned and debarked, ready to replace rotten palisade sections."
)
add_room(
    8, 2, "camp_beast_pen", "Pack-Beast Stable",
    "A fenced corral with hay mangers where thick-furred cave mules rest after hauling heavy wagons.",
    "The mules chew silently on dry hay. The yard smells of sweet grass, manure, and leather harness oil.",
    "the wooden hay mangers", ["mangers", "hay", "stable", "trough"],
    "Long wooden troughs filled with dried prairie hay brought down from the surface."
)
add_room(
    9, 2, "camp_se_tower", "Southeast Watchtower",
    "A tall wooden tower guarding the corner where the camp wall meets the eastern mine boundary.",
    "A large iron alarm bell hangs from the timber crossbeam, overlooking the entire lower valley.",
    "the iron alarm bell", ["bell", "alarm bell", "clapper", "rope"],
    "A big iron bell cast with warning runes, ready to ring if trouble breaks out in the mines."
)

# ==============================================================================
# ZONE 1: THE VILLAGE (y = 3..9, x = 0..4) - 35 rooms
# ==============================================================================

# Row 3 (Village approach & cliff ledges)
add_room(
    0, 3, "village_nw_overlook", "High Stone Overlook",
    "A rocky ledge high above the village roofs. You can see the pale yellow lantern lights of town below.",
    "The cave wall curves sharply here. Cool water drips into a small mossy pool from cracks in the ceiling.",
    "the mossy drip pool", ["pool", "water", "moss", "drip"],
    "A clear stone basin filled with cold runoff water, edged with pale green lichen."
)
add_room(
    1, 3, "village_watch_ledge", "North Cave Ledge",
    "A narrow stone path winding along the north cave wall. Stone handrails have been carved into the living rock.",
    "Looking down, you can see smoke rising from village hearths. The air smells of wood ash and clean stone.",
    "the carved handrail", ["handrail", "stone", "rail", "ledge"],
    "Smooth rock carved by decades of miners' hands walking along the upper path."
)
add_room(
    2, 3, "village_north_path", "North Cliff Path",
    "A broad gravel path sloping downward toward the village streets. Small lanterns hang from iron spikes in the rock.",
    "The path is well-traveled, swept clear of loose stones so walking miners will not twist their ankles.",
    "the iron spike lanterns", ["lanterns", "spikes", "lights", "lantern"],
    "Small tin lanterns burning tallow candles, casting warm circles of light on the gravel."
)
add_room(
    3, 3, "village_fort_slope", "Guard Camp Slope",
    "A steep rocky slope leading up toward the dark timber walls of the soldiers' fort.",
    "Warning signs carved on wooden boards remind commoners not to approach the palisade without permission.",
    "the wooden warning sign", ["sign", "warning", "board", "post"],
    "A weather-beaten board painted with a red shield and the words: 'HALT - SOLDIERS ONLY'."
)
add_room(
    4, 3, "village_approach_road", "Camp Approach Road",
    "A wide dirt road running north toward the heavy gates of the Guard Camp.",
    "Wheel ruts from supply wagons cut deep into the ground. Torchlight from the guard towers flickers overhead.",
    "the deep wagon ruts", ["ruts", "road", "wheel ruts", "tracks"],
    "Hardened clay grooves worn by heavy wagons carrying iron, food, and mining gear to the soldiers."
)

# Row 4 (Upper village terrace & northern residential lane)
add_room(
    0, 4, "village_upper_terrace", "Upper Residential Terrace",
    "A quiet stone terrace where several elder families have their homes. Low stone walls border the walkway.",
    "Flowerpots made of hollowed river rocks hold pale subterranean flowers that open in the dark.",
    "the stone flowerpots", ["pots", "flowerpots", "flowers", "plants"],
    "Rough stone cups growing small white cave blossoms that smell faintly like vanilla."
)
add_room(
    1, 4, "village_stone_steps", "Carved Stone Steps",
    "Broad stone stairs cut straight into the natural bedrock, linking the upper terrace to the streets below.",
    "The center of each step is worn smooth and concave from thousands of boots climbing up and down.",
    "the worn steps", ["steps", "stairs", "bedrock", "stone"],
    "Heavy stone steps smoothed down by generations of miners walking home from long shifts."
)
add_room(
    2, 4, "village_north_street", "North Village Street",
    "A cobblestone lane lined with modest stone huts. Pale curtains hang in the windows of miner homes.",
    "You can hear people talking quietly inside. The smell of boiling cabbage and baked bread fills the air.",
    "the cobblestone paving", ["cobblestones", "street", "stones", "paving"],
    "Rounded river stones set into clay to make a clean, dry street through the town."
)
add_room(
    3, 4, "village_carpenters_shed", "Miner Tool Shed",
    "An open wooden shed filled with replacement pick handles, shovel blades, and wooden wheelbarrows.",
    "Sawdust covers the dirt floor. An old carpenter works on repairing a split wooden wheelbarrow.",
    "the rack of pick handles", ["handles", "picks", "tools", "wood"],
    "Smooth hickory handles shaved and oiled, ready to be fitted to iron pickaxe heads."
)
add_room(
    4, 4, "village_east_fork", "East Village Fork",
    "A three-way fork in the cobblestone road. Signs point west to the square and east toward the mines.",
    "Miners with dusty faces and iron buckets pass by, returning from their ten-hour digging shifts.",
    "the wooden direction signs", ["signs", "post", "pointer", "fork"],
    "A painted post pointing west toward 'Town Square' and east toward 'Mana Quarry'."
)

# Row 5 (Core town row: Elder Hut, Village Hall, Village Square, Market Lane, Mine Crossing Road)
add_room(
    0, 5, "elder_hut", "The Elder's Hut",
    "A small, tidy stone hut lined with shelves of old village records. A stone hearth burns low in the corner.",
    "Elder Valen sits at a low pine table, looking at an official scroll sealed with red temple wax. His hands tremble slightly, and he looks exhausted and guilty.",
    "the elder's table", ["table", "hearth", "scroll", "papers", "desk"],
    "A heavy pine table covered in rosters of villagers. A temple document with a broken wax seal lies open in the center."
)
add_room(
    1, 5, "village_hall", "The Village Hall",
    "A large wooden hall built against the natural rock wall. Long benches and tables fill the room, where miners eat soup together after long shifts.",
    "The air smells of mushroom soup, wood smoke, and damp wool. An iron soup kettle hangs over a central fire pit. Villagers sit in small groups, whispering angrily about Kip's arrest.",
    "the central fire pit", ["fire", "hearth", "kettle", "pit", "soup"],
    "A wide stone circle filled with glowing embers. A giant black iron kettle bubbles with hot mushroom and bean broth."
)
add_room(
    2, 5, "village_square", "The Village Square",
    "A broad open square at the center of the underground village. A stone well sits in the middle, and an iron fire bowl provides warm yellow light.",
    "Stone cottages crowd against the cavern walls. To the west is the village dining hall and elder's home. To the east, cart tracks lead toward the mana mines. To the north, a gated road leads to the soldier camp.",
    "the iron fire bowl", ["fire bowl", "bowl", "fire", "well", "flames"],
    "A massive iron cauldron on three stone legs, filled with slow-burning peat that warms the center of town."
)
add_room(
    3, 5, "village_market_lane", "Lantern Lane",
    "A bustling lane where village traders spread cloth blankets to barter woven blankets, clay bowls, and salt.",
    "Paper lanterns hang on strings across the street. Children run between the stalls playing tag.",
    "the market trade blankets", ["blankets", "stalls", "goods", "pots"],
    "Woven wool blankets laid out with handmade clay mugs, dried mushrooms, and carved wooden toys."
)
add_room(
    4, 5, "village_mine_gate", "Mine Crossing Road",
    "A wide stone-paved road connecting the village directly to the mine works. Heavy wooden carts rumble past.",
    "The air begins to smell of stone dust and sharp static electricity. A wooden arch marks the entrance to the mining zone.",
    "the timber entrance arch", ["arch", "timber", "posts", "sign"],
    "A stout archway made of squared mine timbers, carved with the crossed pickaxe badge of the diggers."
)

# Row 6 (Bakery row, well square, and weaver huts)
add_room(
    0, 6, "village_west_cottage", "Miner Cottage Row West",
    "A row of neat stone cottages built right into hollows in the cavern wall. Small wooden fences enclose little yards.",
    "Lines of washed clothes hang between stone pillars. An old woman sits on a bench mending a wool jacket.",
    "the clothesline", ["clothesline", "laundry", "clothes", "rope"],
    "Hemp rope strung between stone pins, holding drying wool work shirts and denim trousers."
)
add_room(
    1, 6, "baker_hut", "Bess's Bakery",
    "A warm, inviting stone cottage with a domed brick oven built into the rock wall. Fresh mushroom bread cools on pine shelves.",
    "Bess the baker stands by the counter with flour on her apron. Her eyes are red from crying about Kip. The smell of sweet yeast and toasted grain fills the room.",
    "the domed brick oven", ["oven", "brick", "hearth", "counter", "shelves"],
    "A large rounded oven with glowing embers inside, baking loaves of dark, hearty cave bread."
)
add_room(
    2, 6, "village_well_square", "The Village Well",
    "A small plaza centered on a circular stone well that taps into an underground mountain stream.",
    "A wooden crank and copper bucket let villagers haul up fresh, icy water. Women gather here to fill clay jugs.",
    "the circular stone well", ["well", "crank", "bucket", "water"],
    "Deep stone masonry reaching down to cold, sweet subterranean water forty feet below."
)
add_room(
    3, 6, "village_weaver_hut", "Weaver's Workshop",
    "A wide cottage where large wooden looms clack rhythmically. Baskets of spun mushroom fiber and sheep wool crowd the floor.",
    "A weaver uses a wooden shuttle to thread dark blue yarn through a hanging frame. Tough work coats hang ready for sale.",
    "the timber loom", ["loom", "threads", "frame", "shuttle"],
    "A sturdy oak weaving frame strung with hundreds of tough gray cords made of cave fungus fiber."
)
add_room(
    4, 6, "village_east_cottage", "Miner Cottage Row East",
    "Stone houses built along the eastern cliff. Children play with smooth crystal pebbles on the doorsteps.",
    "The ground here is tinged slightly violet from dust carried on the boots of returning family members.",
    "the crystal pebbles", ["pebbles", "stones", "crystals", "toys"],
    "Small round stones with faint purple veins, gathered by children from the quarry wash."
)

# Row 7 (Mushroom beds row, school, spore sheds)
add_room(
    0, 7, "village_southwest_lane", "Quiet Cul-de-sac",
    "A quiet dead-end alley between steep stone walls. Solitary stone benches offer a place to think.",
    "A faint draft of warm air rises from fissures in the stone. Small bats roost high in the ceiling cracks.",
    "the stone bench", ["bench", "stone", "seat", "fissure"],
    "A slab of slate resting on two granite blocks, worn smooth by generations of quiet resting."
)
add_room(
    1, 7, "village_school_room", "Village School Room",
    "A simple wooden room with rows of low desks and slate boards where village children learn to read and calculate.",
    "Chalk sticks and slate erasers sit in wooden trays. A painted chart on the wall shows different types of rocks and crystals.",
    "the mineral wall chart", ["chart", "board", "slate", "desks"],
    "A linen poster showing drawings of iron ore, granite, quartz, and glowing mana crystals."
)
add_room(
    2, 7, "mushroom_beds", "The Stepped Mushroom Beds",
    "Stepped garden terraces cut into the cavern floor where villagers grow food in the dark. Thousands of thick mushrooms sprout from damp mulch beds.",
    "A gentle mist falls from pipes in the ceiling. The garden smells of rich black soil and damp earth. Soft blue glow-caps provide just enough light to see rows of edible brown and gold fungi.",
    "the glowing mushroom terraces", ["terrace", "garden", "mushrooms", "beds", "fungus"],
    "Raised rock beds filled with dark compost and decaying timber, covered in clusters of plump edible mushrooms."
)
add_room(
    3, 7, "village_spore_drying", "Spore Drying Shed",
    "A warm timber shed with open lattice walls where mushroom spores and caps are dried on linen screens.",
    "The air is warm and smells rich like roasted nuts. Wooden trays are stacked from floor to ceiling.",
    "the drying screens", ["screens", "trays", "spores", "mushrooms"],
    "Square wooden frames covered with stretched cheesecloth, holding thousands of tiny drying spore clusters."
)
add_room(
    4, 7, "village_lower_path", "Low Path to Lower Mines",
    "A sloped dirt track running down toward the lower crushers and tailings piles.",
    "Miners wearing thick leather aprons carry baskets of raw rock up toward the processing sheds.",
    "the sloped dirt track", ["path", "track", "slope", "road"],
    "A wide ramp packed hard by boots and wheelbarrows, dusted with fine white rock powder."
)

# Row 8 (Springs, washing basins, and root cellars)
add_room(
    0, 8, "village_water_spring", "Clean Water Spring",
    "A natural freshwater spring bubbling up through a fracture in white limestone rock.",
    "The water is crystal clear and ice cold. Moss and pale green water-cress grow around the pool's stone lip.",
    "the bubbling spring", ["spring", "water", "pool", "limestone"],
    "A natural spring pouring dozens of gallons of pure water every minute into a natural stone basin."
)
add_room(
    1, 8, "village_wash_basin", "Communal Wash Basin",
    "A long stone trough fed by the spring runoff where villagers wash work shirts, blankets, and iron pots.",
    "Wooden washboards and bars of brown tallow soap sit on the stone ledge. The water flows clean toward the south drain.",
    "the stone wash trough", ["trough", "basin", "washboard", "soap"],
    "A thirty-foot stone channel cut with grooves where laundry is scrubbed clean."
)
add_room(
    2, 8, "village_south_lane", "South Mushroom Lane",
    "A quiet walkway bordering the southern edge of the mushroom gardens. Small stone sheds hold mulch and tools.",
    "The smell of rich black soil is strongest here. Earthworms crawl in the compost beds along the wall.",
    "the tool storage shed", ["shed", "tools", "rakes", "shovels"],
    "A low shed holding wooden rakes, iron trowels, and watering pots for tending the fungus beds."
)
add_room(
    3, 8, "village_compost_shed", "Garden Compost Shed",
    "A large stone bin holding decaying wood chips, straw, and spent mushroom compost.",
    "The compost generates natural heat, warming the southern corner of the village. The soil is dark and crumbly.",
    "the steaming compost bin", ["compost", "bin", "soil", "chips"],
    "A deep stone enclosure where decaying plant matter turns into rich soil for the crops."
)
add_room(
    4, 8, "village_root_cellar", "Village Root Cellar",
    "A cool, dry cave room lined with wooden bins holding stored turnips, pickled mushrooms, and dried roots.",
    "The village keeps its winter emergency food here. The air smells of earth, vinegar, and dry straw.",
    "the food storage bins", ["bins", "cellar", "turnips", "roots"],
    "Deep slatted pine bins filled with sweet yellow turnips and dried fungus cakes."
)

# Row 9 (Southern overlooks, memorial stones, deep storage)
add_room(
    0, 9, "village_sw_corner", "South Cliff Basin",
    "The southwestern-most corner of the cavern. The rock wall drops into an old natural chasm below.",
    "A sturdy timber railing prevents anyone from slipping over the edge. Cool air drafts upward from unseen depths.",
    "the cliff edge railing", ["railing", "fence", "cliff", "chasm"],
    "Double timber rails bolted into iron eyelets anchored deep into the cliff bedrock."
)
add_room(
    1, 9, "village_memorial_stones", "Miner Memorial Stones",
    "A quiet alcove where dozens of small slate stones have been set into the cave wall.",
    "Each stone is carved with the name of a miner who died in cave-ins or gas leaks over the past sixty years. Small tallow candles burn in front of many stones.",
    "the carved memorial stones", ["stones", "memorial", "names", "candles"],
    "Dozens of small slate tablets etched with names, dates, and pickaxe emblems to honor fallen workers."
)
add_room(
    2, 9, "village_south_overlook", "South Cave Ledge",
    "A high natural shelf offering a view of the southern cavern wall where natural mineral streaks sparkle like stars.",
    "Faint trickles of water trace glistening lines down the black stone face. The space is quiet and peaceful.",
    "the glistening mineral streaks", ["minerals", "wall", "streaks", "quartz"],
    "Bands of natural quartz and calcite embedded in the dark limestone that catch the candle glow."
)
add_room(
    3, 9, "village_storage_caves", "Deep Village Storehouse",
    "A deep natural chamber used to store spare building stone, rolls of canvas, and wooden barrel staves.",
    "Cobwebs hang from the ceiling in thick sheets. The room is quiet and undisturbed.",
    "the stacks of building stone", ["stone", "blocks", "canvas", "crates"],
    "Squarish blocks of limestone cut from the quarry, stacked ready for chimney and wall repairs."
)
add_room(
    4, 9, "village_drain_trench", "Southern Runoff Trench",
    "A stone trench cut into the floor to funnel runoff water out of the village toward the lower mine sluices.",
    "Water rushes steadily down the channel, carrying loose dirt and fine sand toward the quarry drainage.",
    "the stone runoff trench", ["trench", "channel", "water", "drain"],
    "A two-foot-wide gutter paved with smooth river pebbles, directing village wastewater away from living areas."
)

# ==============================================================================
# ZONE 2: THE MANA MINE (y = 3..9, x = 5..9) - 35 rooms
# ==============================================================================

# Row 3 (Drainage, pump station, north ventilation)
add_room(
    5, 3, "old_drain_pipe", "The Old Drain Pipe",
    "A narrow, low tunnel lined with cracked clay tiles. Cold water trickles over your boots, running north under the stone foundations of the guard camp.",
    "You have to crouch low to move through the pipe. Slimy green moss covers the damp clay walls. Up ahead, faint light and the sound of soldiers' voices leak through a loose wooden floor hatch.",
    "the wooden floor hatch", ["hatch", "ceiling", "opening", "trapdoor", "boards"],
    "Square wooden planks in the ceiling above. You can smell lamp oil and hear heavy boots pacing back and forth above you."
)
add_room(
    6, 3, "mine_pump_station", "Mine Drainage Pump",
    "A clanking mechanical pump powered by an iron water wheel. Wooden pistons pump sludge out of the low tunnels.",
    "The noise is deafening: *thump-clank... thump-clank*. Wet sludge splashes into a flume that empties into the drainage network.",
    "the wooden pump pistons", ["pump", "pistons", "wheel", "engine"],
    "Heavy oak beams linked to an iron crank, rising and falling with rhythmic mechanical force."
)
add_room(
    7, 3, "mine_north_shaft", "North Ventilation Shaft",
    "A high vertical chimney cut into the rock ceiling. Fresh, cold air rushes down from upper levels, cooling the humid quarry.",
    "A canvas wind-sail hangs suspended on iron ropes, catching air and directing it into the deeper tunnels.",
    "the canvas wind-sail", ["wind-sail", "sail", "shaft", "chimney"],
    "Heavy sailcloth stretched over an iron hoop, channeling a steady stream of crisp surface air downward."
)
add_room(
    8, 3, "mine_blasting_storage", "Blasting Powder Store",
    "A heavily reinforced vault behind thick oak doors. Red barrels of mineral blasting powder sit on dry wooden racks.",
    "Warning symbols of a bursting rock are painted on the doors. A brass lock keeps unauthorized workers out.",
    "the powder barrels", ["barrels", "powder", "kegs", "explosives"],
    "Stout oak kegs sealed with black pitch, holding fine volcanic sulfur powder for breaking hard rock."
)
add_room(
    9, 3, "mine_northeast_drift", "High Crystal Drift",
    "A narrow exploration drift following a thin seam of sparkling amethyst crystals.",
    "Chisel marks cover every inch of the wall. Tiny purple sparks jump between crystal needles when you brush against them.",
    "the needle crystal seam", ["crystals", "seam", "needles", "amethyst"],
    "Thin, razor-sharp needles of violet crystal clustering along a narrow fault line in the stone."
)

# Row 4 (Drainage culvert, cart depot, timber supports, bright crystal seams)
add_room(
    5, 4, "mine_drainage_culvert", "Flooded Culvert",
    "A knee-deep water channel running between the village runoff and the mine drainage. Water splashes over slippery flagstones.",
    "A rusty iron grate in the north wall leads into the old drain pipe. The air is cool and smells of mineral salts.",
    "the rusty iron grate", ["grate", "bars", "culvert", "drain"],
    "An iron grate whose lower bars have rotted through from years of mineral-rich water, leaving room to slip through."
)
add_room(
    6, 4, "mine_cart_depot", "Mine Cart Staging Depot",
    "A wide junction where multiple iron rail lines merge into a turntable. Empty wooden ore carts wait in neat rows.",
    "Grease pots and iron levers sit by the track switch. A blackboard lists the daily cart quotas for each mining crew.",
    "the iron rail turntable", ["turntable", "tracks", "switch", "rails"],
    "A revolving steel disc set into the floor that allows heavy carts to turn and change tracks easily."
)
add_room(
    7, 4, "mine_timber_support", "Heavy Timber Support Hall",
    "Massive oak archways support the cracked cavern ceiling here. The timbers creak softly under the weight of the stone above.",
    "Wedges of cedar are hammered into every joint. Miners pass through this corridor quickly, keeping their eyes on the ceiling.",
    "the creaking oak timbers", ["timbers", "arch", "beams", "wood"],
    "Foot-thick square oak beams bolted together with iron bands, holding back thousands of tons of fractured stone."
)
add_room(
    8, 4, "mine_crystal_seam_north", "Bright Violet Vein",
    "A stunning wall of solid violet mana crystals exposed by recent excavation. The crystals glow with steady cold light.",
    "The light is bright enough to read by without a candle. You can feel a faint vibration in your teeth as magic pulses through the rock.",
    "the solid violet crystal vein", ["vein", "crystals", "light", "violet"],
    "A continuous ribbon of pure purple crystal three feet wide, pulsing with natural magical electricity."
)
add_room(
    9, 4, "mine_east_chute", "Ore Loading Chute",
    "A sloped wooden chute built against a cliff face. Workers above tip wheelbarrows of raw crystal ore down into waiting carts below.",
    "Rock chunks clatter down the wooden slide with a sound like thunder. A thick cloud of white stone dust hangs in the air.",
    "the wooden ore chute", ["chute", "slide", "timber", "trough"],
    "A long slide lined with sheet iron so rock chunks slide smoothly down into the haulage carts."
)

# Row 5 (Core mine row: Mine Entrance, Cart Tracks, Main Drift, Gallery, Eastern Stope)
add_room(
    5, 5, "mine_entrance", "The Mine Entrance",
    "A wide archway cut into the rock wall where the village road meets the excavation tunnels. Sturdy timber beams frame the tunnel mouth.",
    "The air is cool, dry, and smells of pulverized stone and static electricity. Iron tracks disappear into the darkness ahead, and glowing purple crystal seams trace veins across the ceiling.",
    "the timber entrance frame", ["beams", "frame", "arch", "timbers", "entrance"],
    "Massive square oak beams braced with iron plates to keep the tunnel mouth from collapsing."
)
add_room(
    6, 5, "cart_tracks", "The Cart Tracks",
    "A double line of iron rails runs down the center of this broad tunnel. Empty wooden ore carts sit on side spurs waiting to be loaded.",
    "Lanterns hang from ceiling timbers every twenty paces. Miners shout to each other over the clinking of iron picks and the rumble of iron cart wheels.",
    "the wooden ore cart", ["cart", "ore cart", "wagon", "rails", "tracks"],
    "A four-wheeled wagon built from thick oak planks and bound in iron bands. A hand brake lever sits near the front axle."
)
add_room(
    7, 5, "mine_main_drift", "Main Haulage Tunnel",
    "The busiest tunnel in the quarry. Two lines of tracks allow loaded carts to roll west while empty carts return east.",
    "Miners push heavy carts with their shoulders down, their faces coated in blue-gray stone powder. The air hums with crystal energy.",
    "the iron cart rails", ["rails", "tracks", "ties", "iron"],
    "Twin steel bars spiked to wooden ties, polished bright and shiny by hundreds of rolling cart wheels."
)
add_room(
    8, 5, "mine_chiseled_gallery", "Chiseled Stone Gallery",
    "A high vaulted gallery where stone masons have chiseled away the surrounding slate to expose giant crystal pillars.",
    "Pillars of translucent blue and violet crystal stand ten feet tall, glowing like frozen icebergs in the gloom.",
    "the glowing crystal pillars", ["pillars", "crystals", "icebergs", "columns"],
    "Spectacular natural columns of pure mana crystal that illuminate the entire chamber with azure light."
)
add_room(
    9, 5, "mine_eastern_stope", "East Crystal Stope",
    "A stepped excavation room cutting upward into the eastern rock wall. Scaffolding ladders climb twenty feet to working faces.",
    "Miners on the ladders chip away at crystal clusters with small copper chisels, catching the falling shards in canvas aprons.",
    "the timber scaffolding", ["scaffolding", "ladders", "frames", "timbers"],
    "Multi-level wooden frames lashed with rawhide, allowing miners to reach high veins safely."
)

# Row 6 (Sorting shed, quarry rim, Crystal Pit, deep crystal vein, shored-up drift)
add_room(
    5, 6, "mine_sorting_shed", "Ore Sorting Shed",
    "A long open-sided shed where miners use hand hammers to break rock away from raw mana crystals.",
    "Chipped crystal shards fill wooden sorting tubs labeled by purity. The room glows with mixed hues of sapphire and amethyst.",
    "the sorting tubs", ["tubs", "boxes", "crystals", "shards"],
    "Wooden crates filled with graded crystal chunks: dull purple for village lamps, bright blue for high-level spells."
)
add_room(
    6, 6, "mine_quarry_rim", "Quarry Pit Rim",
    "The upper edge of the great excavation pit. A heavy timber guardrail prevents workers from falling into the deep quarry below.",
    "Looking down, you can see dozens of miners working by torchlight thirty feet beneath you. Winch ropes hang over the rim.",
    "the pit guardrail", ["guardrail", "rail", "rim", "edge"],
    "Heavy oak rails secured to iron posts anchored in the rock, guarding the edge of the pit."
)
add_room(
    7, 6, "crystal_pit", "The Crystal Pit",
    "A huge natural cave with a deep pit in the middle. Giant clusters of glowing purple and blue mana crystals stick out from the rock walls like jagged glass.",
    "The air crackles with magic that makes your skin tingle. Bright purple light reflects off wet stone walls. Down in the pit, miners use hand chisels to crack free glowing chunks of crystal.",
    "the glowing mana crystals", ["crystals", "crystal", "mana crystals", "veins", "purple crystals"],
    "Thick spikes of purple crystal that pulse with cold inner light. They store pure magic and power the machines of the upper floors."
)
add_room(
    8, 6, "mine_deep_crystal_vein", "Deep Mana Crystal Seam",
    "A massive deposit of blue and purple crystals cutting through dark granite. The rock seems to hum like a living creature.",
    "Whenever a chisel strikes the wall, rings of soft light ripple through the crystal clusters. The magic here is thick and potent.",
    "the humming crystal cluster", ["cluster", "crystals", "vein", "seam"],
    "A dense cluster of blue crystals that vibrates with a faint, pleasant musical note when tapped."
)
add_room(
    9, 6, "mine_collapsed_drift", "Shored-Up Drift",
    "A tunnel that partially collapsed during a tremor months ago, now reinforced with double rows of timber props.",
    "Loose rubble has been cleared to the sides. Warning marks chalked on the ceiling warn diggers to keep vibration low.",
    "the chalk warning marks", ["chalk", "marks", "symbols", "warning"],
    "White chalk circles indicating fractured ceiling rock that could drop if hit with heavy hammers."
)

# Row 7 (Crushers, winches, golem worksite, blue pockets, tailings)
add_room(
    5, 7, "mine_crusher_floor", "Stone Crusher Floor",
    "Two massive iron stampers driven by overhead water ropes rise and drop onto stone slabs, crushing ore into gravel.",
    "The pounding rhythm shakes the stone under your boots: *CRUNCH... CRUNCH*. Fine dust covers everything in a gray blanket.",
    "the iron ore stampers", ["stampers", "crusher", "pestles", "iron"],
    "Hundreds of pounds of cast iron rising and slamming down to break stone away from stubborn crystal nuggets."
)
add_room(
    6, 7, "mine_winch_house", "Pit Winch Station",
    "A heavy wooden winch frame wound with thick hemp ropes. Iron hooks raise and lower ore buckets into the quarry pit.",
    "Two strong miners turn the iron hand cranks. A leather brake shoe squeals as a loaded bucket reaches the top.",
    "the heavy hemp winch", ["winch", "ropes", "cranks", "drum"],
    "A giant oak drum wrapped in two-inch hemp rope, capable of lifting half a ton of crystal ore."
)
add_room(
    7, 7, "mine_golem_worksite", "Golem Excavation Floor",
    "A deep stone chamber where an old, moss-dusted stone golem works silently, lifting rocks three times heavier than a human could move.",
    "Orin the golem stands patient and still between lifting boulders. His stone eyes are dim and foggy, waiting for someone with enough wisdom to wake his mind.",
    "the silent stone golem", ["golem", "orin", "statue", "stone"],
    "A massive humanoid carved from gray granite, banded with iron rings. He works without complaining, his mind clouded by age."
)
add_room(
    8, 7, "mine_blue_crystal_pocket", "Blue Crystal Pocket",
    "A hollow geode chamber inside the rock wall. The entire interior is lined with sparkling cobalt-blue crystals.",
    "The air is cool and smells clean like mountain snow. Stepping inside feels like walking into an underground sky full of blue stars.",
    "the cobalt crystal geode", ["geode", "crystals", "cobalt", "hollow"],
    "A natural spherical pocket lined with thousands of deep blue crystal points that catch and bend the light."
)
add_room(
    9, 7, "mine_crystal_tailings", "Crushed Rock Tailings",
    "A sloping hill of crushed white quartz and discarded stone tailings dumped from the processing rooms above.",
    "Small pieces of low-grade crystal still glint in the rubble pile. The gravel crunches loudly underfoot.",
    "the pile of crushed tailings", ["tailings", "gravel", "rubble", "pile"],
    "Pounds of shattered quartz gravel left over after the valuable mana crystals have been picked clean."
)

# Row 8 (Lower rail spurs, dumps, deep sump, submerged rails, seep tunnel)
add_room(
    5, 8, "mine_lower_rail_spur", "Lower Rail Spur",
    "A quiet spur track leading into the lower storage tunnels. Old carts with missing wheels sit on the sidelines.",
    "Grease cans and oily rags litter an upturned barrel. A track hand works on tightening loose rail spikes.",
    "the loose rail switch", ["switch", "spur", "tracks", "rails"],
    "A hand-forged iron lever that shifts the rail tracks between the main drift and the repair yard."
)
add_room(
    6, 8, "mine_tailing_dump", "Rock Tailing Dump",
    "A natural ravine where miners dump barren rock that contains no mana crystals.",
    "Dust rises in plumes as a wheelbarrow of gravel is tipped over the wooden lip into the dark gap.",
    "the dumping chute", ["chute", "ravine", "edge", "dump"],
    "A slanted plank chute worn slick by millions of falling stones pouring into the chasm."
)
add_room(
    7, 8, "mine_deep_sump", "Deep Mine Sump",
    "The lowest point of the quarry pit. Water collects here in a black pool before being pumped back to the surface.",
    "A wooden raft floats in the center with an intake pipe. The water is cold and clear, reflecting the violet glow of the walls above.",
    "the floating pump raft", ["raft", "pool", "sump", "pipe"],
    "A buoyant platform made of sealed barrels and planks holding the suction mouth of the drainage pump."
)
add_room(
    8, 8, "mine_submerged_rail", "Submerged Rail Drift",
    "An old mine tunnel where groundwater has risen six inches above the iron tracks, creating a flooded hallway.",
    "Your boots splash through cold, crystal-clear water. Abandoned iron picks rest against the wet stone wall.",
    "the flooded rail tracks", ["water", "tracks", "rails", "flood"],
    "Iron rails running under clear water, rippling with violet reflections from crystals on the ceiling."
)
add_room(
    9, 8, "mine_seep_tunnel", "Crystal Seep Tunnel",
    "A narrow, winding tunnel where water seeps through crystal-encrusted cracks in the ceiling.",
    "Mineral water drips continuously: *plip... plip... plip*. Delicate crystal stalactites hang down like icy fingers.",
    "the crystal stalactites", ["stalactites", "needles", "icicles", "stone"],
    "Hanging points of pure purple crystal formed over centuries by mineral-rich water dripping from the roof."
)

# Row 9 (South sluice, deepest cut, abyssal fault, geode floor, quiet grotto)
add_room(
    5, 9, "mine_south_sluice", "Runoff Sluice",
    "A wide wooden flume that carries wastewater from both the village and the mine crushers toward the south chasm.",
    "The water rushes fast and loud over smooth timber boards. Thick safety ropes are strung along the sides.",
    "the rushing water flume", ["flume", "sluice", "water", "trough"],
    "A stout wooden trough carrying three feet of swiftly flowing water south into the abyss."
)
add_room(
    6, 9, "mine_deepest_cut", "The Deepest Cut",
    "The deepest active excavation trench in the quarry. Huge stepped rock benches descend forty feet below the main floor.",
    "The crystals here are darker and more concentrated, glowing deep royal indigo. The air is warm and heavy with power.",
    "the royal indigo crystals", ["crystals", "indigo", "vein", "trench"],
    "Dense, dark crystal clusters that pulse with immense raw energy, stored deep in the earth."
)
add_room(
    7, 9, "mine_abyssal_fault", "Abyssal Rock Fault",
    "A massive natural fissure where the cave floor splits into a bottomless chasm. Wind howls up from the deep earth.",
    "A rope bridge crosses the narrowest gap, connecting to the southern crystal pockets. The drop below is pitch black.",
    "the abyss rope bridge", ["bridge", "ropes", "fissure", "chasm"],
    "A swaying bridge of braided rope and split pine planks spanning the dizzying dark crevasse."
)
add_room(
    8, 9, "mine_crystal_bed", "Geode Floor",
    "A flat stone room where the floor itself is made of solid, polished crystal. Walking here feels like standing on frozen violet glass.",
    "Your boots make high ringing sounds against the hard crystal floor. Faint geometric patterns crisscross beneath the surface.",
    "the crystalline floor", ["floor", "crystal", "glass", "bed"],
    "A naturally polished sheet of solid quartz and mana crystal, perfectly flat and glowing with internal light."
)
add_room(
    9, 9, "mine_southeast_grotto", "Quiet Crystal Grotto",
    "A peaceful hidden chamber far from the noise of pickaxes and ore carts. Giant sapphire-blue crystals form a natural circle.",
    "A calm pool of water in the center reflects the gentle blue light. Miners come here on their breaks to rest and breathe easy.",
    "the circular crystal ring", ["circle", "crystals", "ring", "pool"],
    "A natural ring of blue crystal spires standing like silent guardians around a still reflective pool."
)

# Verify we have exactly 100 rooms
assert len(ROOM_DATA) == 100, f"Expected 100 rooms, got {len(ROOM_DATA)}"

# Check that all expected landmark rooms are present
EXPECTED_LANDMARKS = [
    "village_square", "village_hall", "elder_hut", "mushroom_beds",
    "mine_entrance", "cart_tracks", "crystal_pit", "old_drain_pipe",
    "camp_gate", "command_tent", "prison_cage", "teleport_gate"
]
for lk in EXPECTED_LANDMARKS:
    found = any(r["id"] == lk for r in ROOM_DATA.values())
    assert found, f"Missing landmark room: {lk}"

print("All 100 room definitions created and all 12 landmarks verified.")

# ==============================================================================
# CONNECT EXITS
# ==============================================================================
# Rules for connections:
# 1. Standard grid connections between adjacent rooms within zones:
#    - Zone 3 (Guard Camp): x in 0..9, y in 0..2
#    - Zone 1 (Village): x in 0..4, y in 3..9
#    - Zone 2 (Mana Mine): x in 5..9, y in 3..9
# 2. Border connections between Zone 1 and Zone 2 (x=4 and x=5, y=3..9):
#    - Along the vertical seam between village and mine, rooms connect East/West!
# 3. Border between Guard Camp (y=2) and lower zones (y=3):
#    - Camp wall blocks all normal passage between y=2 and y=3.
#    - EXCEPTION 1: camp_gate (4, 2) connects South to village_approach_road (4, 3), and village_approach_road connects North to camp_gate.
#    - EXCEPTION 2: old_drain_pipe (5, 3) connects Up into command_tent (4, 1), and command_tent connects Down into old_drain_pipe.
#    - EXCEPTION 3: camp_gate (4, 2) connects North into command_tent (4, 1) with requires_story_var: "camp_gate_open", and command_tent connects South to camp_gate.
# 4. Special connections required by tests:
#    - village_square connects Up to "oh"
#    - command_tent connects to prison_cage, teleport_gate, old_drain_pipe, and camp_gate
#    - old_drain_pipe connects to command_tent and cart_tracks

# Direction helpers
OPPOSITE = {
    "North": "South",
    "South": "North",
    "East": "West",
    "West": "East",
    "Up": "Down",
    "Down": "Up",
}
ALIAS_MAP = {
    "North": ["n"],
    "South": ["s"],
    "East": ["e"],
    "West": ["w"],
    "Up": ["u"],
    "Down": ["d"],
}

# Build adjacency
room_exits = {r["id"]: [] for r in ROOM_DATA.values()}

def add_connection(from_id, to_id, label, aliases=None, menu_label=None, requires_story_var=""):
    ex = {
        "room_id": to_id,
        "label": label,
        "aliases": aliases if aliases is not None else ALIAS_MAP.get(label, [label.lower()]),
        "menu_label": menu_label if menu_label is not None else label,
    }
    if requires_story_var:
        ex["requires_story_var"] = requires_story_var
    room_exits[from_id].append(ex)

def connect_bidirectional(id_a, id_b, label_a_to_b, requires_story_var_a=""):
    label_b_to_a = OPPOSITE[label_a_to_b]
    add_connection(id_a, id_b, label_a_to_b, requires_story_var=requires_story_var_a)
    add_connection(id_b, id_a, label_b_to_a)

# Standard grid adjacency
for (x, y), r in ROOM_DATA.items():
    current_id = r["id"]
    
    # Check East neighbor (x + 1, y)
    if (x + 1, y) in ROOM_DATA:
        east_r = ROOM_DATA[(x + 1, y)]
        # Are both in camp, or both in lower zones (village/mine)?
        # Camp is y <= 2, lower is y >= 3.
        both_in_camp = (y <= 2)
        both_in_lower = (y >= 3)
        if both_in_camp or both_in_lower:
            connect_bidirectional(current_id, east_r["id"], "East")

    # Check South neighbor (x, y + 1)
    if (x, y + 1) in ROOM_DATA:
        south_r = ROOM_DATA[(x, y + 1)]
        # Internal camp connection (y=0 to y=1, or y=1 to y=2)
        if y < 2:
            # (4, 1) command_tent connects South to (4, 2) camp_gate via gated special connection
            if (x, y) != (4, 1):
                connect_bidirectional(current_id, south_r["id"], "South")
        # Lower zones connection (y >= 3 to y+1)
        elif y >= 3:
            connect_bidirectional(current_id, south_r["id"], "South")
        # Boundary between camp and lower zone (y=2 to y=3):
        elif y == 2:
            # ONLY camp_gate at (4, 2) connects South to village_approach_road at (4, 3)
            if (x, y) == (4, 2):
                connect_bidirectional("camp_gate", "village_approach_road", "South")

# Special story connections:
# 1. village_square Up to oh
add_connection("village_square", "oh", "Up to the Heart-Pit", ["up", "u", "pit", "stairs", "heart"], "Up")

# 2. camp_gate North into command_tent (requires camp_gate_open)
add_connection("camp_gate", "command_tent", "Through the Gate into Camp", ["north", "n", "camp", "gate", "inside"], "North", requires_story_var="camp_gate_open")
add_connection("command_tent", "camp_gate", "South to the Camp Gate", ["south", "s", "gate"], "South")

# 3. old_drain_pipe to command_tent
add_connection("old_drain_pipe", "command_tent", "Climb through Hatch into Guard Camp", ["north", "n", "hatch", "climb", "camp", "tent", "up", "u"], "Climb Up")
add_connection("command_tent", "old_drain_pipe", "Down into the Drain Pipe", ["down", "d", "pipe", "hatch"], "Down")

# 4. old_drain_pipe to cart_tracks
add_connection("old_drain_pipe", "cart_tracks", "Up to the Cart Tracks", ["tracks", "rails", "cart"], "Cart Tracks")
add_connection("cart_tracks", "old_drain_pipe", "Down into the Drain Pipe", ["down", "d", "drain", "pipe", "grate"], "Down")

# 5. command_tent to prison_cage and teleport_gate
if not any(e["room_id"] == "prison_cage" for e in room_exits["command_tent"]):
    add_connection("command_tent", "prison_cage", "East to the Prison Cage", ["east", "e", "cage", "prison"], "East")
if not any(e["room_id"] == "command_tent" for e in room_exits["prison_cage"]):
    add_connection("prison_cage", "command_tent", "West to the Command Tent", ["west", "w", "tent", "command"], "West")

if not any(e["room_id"] == "teleport_gate" for e in room_exits["command_tent"]):
    add_connection("command_tent", "teleport_gate", "North to the Teleport Gate", ["north", "n", "gate", "teleport"], "North")
if not any(e["room_id"] == "command_tent" for e in room_exits["teleport_gate"]):
    add_connection("teleport_gate", "command_tent", "South to the Command Tent", ["south", "s", "tent", "command"], "South")

# Build final rooms list for Floor 4
floor4_rooms = []
for (x, y), r in sorted(ROOM_DATA.items()):
    room_obj = {
        "id": r["id"],
        "title": r["title"],
        "summary": r["summary"],
        "inspect_text": r["inspect_text"],
        "features": r["features"],
        "exits": room_exits[r["id"]],
    }
    floor4_rooms.append(room_obj)

print(f"Built {len(floor4_rooms)} Floor 4 rooms with all exits.")

# Load existing rooms.json
with open("content/layla/locales/en/rooms.json", "r") as f:
    existing_rooms = json.load(f)

# Keep Floor 1, Floor 2, Floor 3 rooms
# Floor 1 starts with 'r', Floor 2 with 'd', Floor 3 starts with 'o' (except old_drain_pipe)
non_floor4_rooms = []
for r in existing_rooms:
    rid = r["id"]
    if rid.startswith("r") or rid.startswith("d") or (rid.startswith("o") and rid != "old_drain_pipe"):
        non_floor4_rooms.append(r)

print(f"Retained {len(non_floor4_rooms)} previous floor rooms (Floors 1, 2, 3).")

# Update 'oh' to ensure it has Down exit to village_square
oh_room = next(r for r in non_floor4_rooms if r["id"] == "oh")
oh_exits = [e for e in oh_room["exits"] if e["room_id"] != "village_square"]
oh_exits.append({
    "room_id": "village_square",
    "label": "Down into the Cave Below",
    "aliases": ["down", "d", "descend", "village", "below"],
    "menu_label": "Down",
    "requires_story_var": "elemental_released"
})
oh_room["exits"] = oh_exits

# Combine
all_rooms = non_floor4_rooms + floor4_rooms
with open("content/layla/locales/en/rooms.json", "w") as f:
    json.dump(all_rooms, f, indent=2)
    f.write("\n")

print(f"Wrote {len(all_rooms)} total rooms to content/layla/locales/en/rooms.json.")

# Update maps.json
with open("content/layla/locales/en/maps.json", "r") as f:
    maps = json.load(f)

commoners_map = next(m for m in maps if m["id"] == "the-commoners")
commoners_rooms = []
for (x, y), r in sorted(ROOM_DATA.items()):
    commoners_rooms.append({
        "room_id": r["id"],
        "x": x,
        "y": y
    })
commoners_map["rooms"] = commoners_rooms

with open("content/layla/locales/en/maps.json", "w") as f:
    json.dump(maps, f, indent=2)
    f.write("\n")

print(f"Updated the-commoners map in maps.json with {len(commoners_rooms)} rooms.")
