global percent = 1 / 100
global second = 1
global minute = 60 * second
global hour = 60 * minute
global W = 1
global kW = 1000 * W
global kJ = kW * second
global MW = 1000 * kW
global MJ = MW * second

global enum FuelType
    "burnable"
    "electricity"
end

global degrees = 1

global enum AssemblerType
    "furnace"
    "assembler"
end

global record AmountOfItem
    name: string
    amount: number
end

global record Recipe
    ingredients: {AmountOfItem}
    produces: {AmountOfItem}
    crafting_time: number
    assembler_type: AssemblerType
end

global recipes: {string:Recipe} = {
    iron_plate = {
        ingredients = { { name = "iron_ore", amount = 1 } },
        produces = { {name = "iron_plate", amount = 1 } },
        crafting_time = 3.2 * second,
        assembler_type = "furnace",
    },
    gear = {
        ingredients = { { name = "iron_plate", amount = 3 } },
        produces = { {name = "gear", amount = 1 } },
        crafting_time = 0.5 * second,
        assembler_type = "assembler",
    },
    green_chip = {
        ingredients = {
            { name = "iron_plate", amount = 2 },
            { name = "copper_wire", amount = 8 },
        },
        produces = { { name = "green_chip", amount = 1 } },
        crafting_time = 0.5 * second,
        assembler_type = "assembler",
    },
    copper_wire = {
        ingredients = { { name = "copper_plate", amount = 1 } },
        produces = { {name = "copper_wire", amount = 2 } },
        crafting_time = 0.5 * second,
        assembler_type = "assembler",
    },
    red_science = {
        ingredients = {
            { name = "copper_plate", amount = 1 },
            { name = "gear", amount = 1 },
        },
        produces = { {name = "red_science", amount = 1 } },
        crafting_time = 5 * second,
        assembler_type = "assembler",
    },
    yellow_belt = {
        ingredients = {
            { name = "iron_plate", amount = 1 },
            { name = "gear", amount = 1 },
        },
        produces = { {name = "yellow_belt", amount = 1 } },
        crafting_time = 0.5 * second,
        assembler_type = "assembler",
    },
    yellow_inserter = {
        ingredients = {
            { name = "iron_plate", amount = 1 },
            { name = "gear", amount = 1 },
            { name = "green_chip", amount = 1 },
        },
        produces = { {name = "yellow_inserter", amount = 1 } },
        crafting_time = 0.5 * second,
        assembler_type = "assembler",
    },
    green_science = {
        ingredients = {
            { name = "yellow_inserter", amount = 1 },
            { name = "yellow_belt", amount = 1 },
        },
        produces = { {name = "green_science", amount = 1 } },
        crafting_time = 6 * second,
        assembler_type = "assembler",
    },
    copper_plate = {
        ingredients = { { name = "copper_ore", amount = 1 } },
        produces = { {name = "copper_plate", amount = 1 } },
        crafting_time = 3.2 * second,
        assembler_type = "furnace",
    },
    steel = {
        ingredients = { { name = "iron_plate", amount = 10 } },
        produces = { { name = "steel", amount = 1 } },
        crafting_time = 32 * second,
        assembler_type = "furnace",
    },
    yellow_ammo = {
        ingredients = { { name = "iron_plate", amount = 4 } },
        produces = { { name = "yellow_ammo", amount = 1 } },
        crafting_time = 1 * second,
        assembler_type = "assembler",
    },
    red_ammo = {
        ingredients = {
            { name = "copper_plate", amount = 5 },
            { name = "steel", amount = 1 },
            { name = "yellow_ammo", amount = 1 },
        },
        produces = { { name = "red_ammo", amount = 1 } },
        crafting_time = 3 * second,
        assembler_type = "assembler",
    },
}

global enum DamageType
    "Physical"
end

global record Damage
    value: number
    type: DamageType
end

global record AmmoType
    magazine_size: number
    damage: {Damage}
end

global record Drill
    mining_speed: number
    pollution: number
    mining_area_side: number
    fuel_type: FuelType
    max_consumption: number
end

global record Assembler
    crafting_speed: number
    fuel_type: FuelType
    max_consumption: number
    pollution: number
end

global record BurnableFuel
    fuel_value: number
end

global stuff = {
    ammo: {string:AmmoType} = {
        yellow_ammo = {
            magazine_size = 10,
            damage = {{ value = 5, type = "Physical" }}
        },
        red_ammo = {
            magazine_size = 10,
            damage = {{ value = 8, type = "Physical" }}
        },
    },
    burnable: {string:BurnableFuel} = {
        wood = {
            fuel_value = 2 * MJ
        },
        coal = {
            fuel_value = 4 * MJ
        },
    },
    drills: {string:Drill} = {
        burner_mining_drill: Drill = {
            mining_speed = 0.25 / second,
            pollution = 12 / minute,
            mining_area_side = 2,
            fuel_type = "burnable",
            max_consumption = 150 * kW,
        },
        electric_mining_drill: Drill = {
            mining_speed = 0.5 / second,
            pollution = 10 / minute,
            mining_area_side = 5,
            fuel_type = "electricity",
            max_consumption = 90 * kW,
        },
    },
    furnaces: {string:Assembler} = {
        stone_furnace = {
            crafting_speed = 1,
            pollution = 2 / minute,
            max_consumption = 90 * kW,
            fuel_type = "burnable",
        },
    },
    boiler = {
        max_consumption = 1.8 * MW,
        pollution = 30 / minute,
        consumes_water = 60 / second,
        generates_steam = 60 / second,
        temperature = 165 * degrees,
    },
    steam_engine = {
        consumes_steam = 30 / second,
        max_temperature = 165 * degrees,
        produces_electricity = true,
        max_output = 900 * kW,
    },
    gray_assembler: Assembler = {
        min_consumption = 2.5 * kW,
        max_consumption = 77.5 * kW,
        pollution = 4 / minute,
        crafting_speed = 0.5,
        fuel_type = "electricity",
    },
}

global record Resistance
    flat: number
    percentage: number
end

global record Biter
    health: number
    spawn_pollution: number
    resistances: {DamageType:Resistance}
end

global biters: {string:Biter} = {
  small_biter = {
    health = 15,
    spawn_pollution = 3.2,
    resistances = {
      Physical = { flat = 0, percentage = 0 * percent }
    }
  },
  medium_biter = {
    health = 75,
    spawn_pollution = 20,
    resistances = {
      Physical = { flat = 4, percentage = 10 * percent }
    }
  },
  big_biter = {
    health = 375,
    spawn_pollution = 80,
    resistances = {
      Physical = { flat = 8, percentage = 10 * percent }
    }
  },
}

global biter_type_used: string
global burnable_fuel_used = "coal"
global drill: Drill
global assembler_to_use: {AssemblerType:Assembler} = {}
global world = {
    biters: {string:number} = {},
    pollution: number = 0,
    items: {string:number} = {},
    total_produced: {string:number} = {}
}

function world.add_item(item: string, amount: number)
    local current = world.items[item]
    if current == nil then
        current = 0
    end
    world.items[item] = current + amount
    if amount > 0 then
        local current_total = world.total_produced[item]
        if current_total == nil then
            current_total = 0
        end
        world.total_produced[item] = current_total + amount
    end
end

for biter_type, _ in pairs(biters) do
    world.biters[biter_type] = 0
end
global produce: {string:function(number)} = {}

global mine: function(item: string, amount: number)

global ensure_have = function(item: string, amount: number)
    local current = world.items[item]
    if current == nil then
        current = 0
    end
    if current < amount then
        produce[item](amount - current)
    end
end

global produce_burnable_fuel_power = function(joules: number)
    local fuel_amount = joules / stuff.burnable.coal.fuel_value
    mine(burnable_fuel_used, fuel_amount)
    world.add_item(burnable_fuel_used, -fuel_amount)
end

global produce_electricity = function(joules: number)
  local engine = stuff.steam_engine
  local steam_engine_time = joules / engine.max_output
  local need_steam = steam_engine_time * engine.consumes_steam
  local boiler = stuff.boiler
  local need_water = need_steam / boiler.generates_steam * boiler.consumes_water
  local boiler_time = need_water / boiler.consumes_water
  world.pollution = world.pollution + boiler_time * boiler.pollution
  produce_burnable_fuel_power(boiler_time * boiler.max_consumption)
end

global produce_power = function(fuel_type: FuelType, joules: number)
    if joules < 1e-5 then
        return
    end
    if fuel_type == "burnable" then
        produce_burnable_fuel_power(joules)
    elseif fuel_type == "electricity" then
        produce_electricity(joules)
    else
        error("HUH")
    end
end

global mine = function(item: string, amount: number)
    local time = amount / drill.mining_speed
    world.pollution = world.pollution + time * drill.pollution
    produce_power(drill.fuel_type, time * drill.max_consumption)

    world.add_item(item, amount)
end

global craft = function(recipe: Recipe, amount: number)
    for _, ingredient in ipairs(recipe.ingredients) do
        local ingredient_amount = ingredient.amount * amount
        ensure_have(ingredient.name, ingredient_amount)
        world.add_item(ingredient.name, -ingredient_amount)
    end
    for _, item in ipairs(recipe.produces) do
        local item_amount = item.amount * amount
        world.add_item(item.name, item_amount)
    end
    local assembler = assembler_to_use[recipe.assembler_type]
    local time = recipe.crafting_time / assembler.crafting_speed
    world.pollution = world.pollution + time * assembler.pollution
    produce_power(assembler.fuel_type, assembler.max_consumption * time)
end

global produce_with_crafting = function(recipe: Recipe, item: string, amount: number)
    print("crafting", amount, "of", item)
    for _, produced_item in ipairs(recipe.produces) do
        if produced_item.name == item then
            craft(recipe, amount / produced_item.amount)
            return
            -- TODO: leftovers?
        end
    end
    error("item not found in recipe")
end

global pollution_into_biters = function()
    local spawned = math.floor(world.pollution / biters[biter_type_used].spawn_pollution)
    -- print("spawning", spawned, biter_type_used)
    world.biters[biter_type_used] = world.biters[biter_type_used] + spawned
    world.pollution = world.pollution - spawned * biters[biter_type_used].spawn_pollution
end

global ammo_used: string
global damage_multiplier: number = 1

global kill_biters = function()
    local ammo = stuff.ammo[ammo_used]
    for biter_type, biter_amount in pairs(world.biters) do
        local biter = biters[biter_type]
        local bullet_damage: number = 0
        for _, damage in ipairs(ammo.damage) do
            local resistance: Resistance = biter.resistances[damage.type]
            if resistance == nil then
                resistance = { flat = 0, percentage = 0 }
            end
            bullet_damage = bullet_damage + math.max(1, damage.value * damage_multiplier - resistance.flat) * (1 - resistance.percentage)
        end
        local magazine_damage = ammo.magazine_size * bullet_damage
        local total_health = biter.health * biter_amount
        local ammo_amount = total_health / magazine_damage
        ensure_have(ammo_used, ammo_amount)
        world.add_item(ammo_used, -ammo_amount)
        world.biters[biter_type] = 0
    end
end

global print_world: function()

global kill_pollution = function()
    local run = true
    while run do
        run = false
        -- print_world()
        local pollution_before = world.pollution
        pollution_into_biters()
        for _, amount in pairs(world.biters) do
            if amount > 0 then
                run = true
            end
        end
        kill_biters()
        if world.pollution > pollution_before then
            error("killing biters only made even more pollution KEKW")
        end
    end
end

global print_world = function()
    print("====")
    print("pollution: ", world.pollution)
    print(" == biters:")
    for biter, amount in pairs(world.biters) do
        print(biter, ": ", amount)
    end
    print(" == items:")
    for item, amount in pairs(world.items) do
        print(item, ": ", amount)
    end
    print(" == total produced:")
    for item, amount in pairs(world.total_produced) do
        print(item, ": ", amount)
    end
end

for _, item in ipairs {"iron_ore", "coal", "copper_ore"} do
    produce[item] = function(amount: number)
        mine(item, amount)
    end
end
for _, recipe in pairs(recipes) do
    for _, item in ipairs(recipe.produces) do
        produce[item.name] = function(amount: number)
            produce_with_crafting(recipe, item.name, amount)
        end
    end
end

global damage_upgrades = 0
global research_damage_upgrade = function()
    damage_upgrades = damage_upgrades + 1
    damage_multiplier = math.pow(1 + damage_upgrades * 0.1, 2)
end
ammo_used = "yellow_ammo"
-- ammo_used = "red_ammo"
-- biter_type_used = "small_biter"
biter_type_used = "medium_biter"
-- biter_type_used = "big_biter"
assembler_to_use.furnace = stuff.furnaces.stone_furnace
assembler_to_use.assembler = stuff.gray_assembler
drill = stuff.drills.burner_mining_drill
-- drill = stuff.drills.electric_mining_drill

produce.red_science(10 + 120 + 300 + 200 + 480)
produce.green_science(480)
kill_pollution()
print_world()

-- drill_pollution = burner_drill_pollution
-- biter = biters.medium_biter
-- -- drill_pollution = electric_drill_pollution

-- iron = 1000
-- defense_iron = iron_to_kill_pollution(smelt_pollution(recipes.iron_plate, iron))
-- print(defense_iron / iron * 100, "% of iron needs to go for making ammo")
