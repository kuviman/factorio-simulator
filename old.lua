second = 1
minute = 60 * second
hour = 60 * minute
W = 1
kW = 1000 * W
kJ = kW * second
MW = 1000 * kW
MJ = MW * second
burnable_fuel = "burnable_fuel"
electricity = "electricity"
degrees = 1
any_furnace = "any_furnace"

recipes = {
  iron_plate = {
    ingredients = { { name = "iron_ore", amount = 1 } },
    produces = { name = "iron_plate", amount = 1 },
    crafting_time = 3.2 * second,
    crafted_in = any_furnace,
  },
  yellow_ammo = {
    ingredients = { { name = "iron_plate", amount = 4 } },
    produces = { name = "yellow_ammo", amount = 1 },
    crafting_time = 1 * second,
    crafted_in = any_furnace,
  },
}

stuff = {
  yellow_ammo = {
    magazine_size = 10,
    damage = 5,
    damage_type = "physical",
  },
  wood = {
    fuel_value = 2 * MJ,
  },
  coal = {
    fuel_value = 4 * MJ,
  },
  burner_mining_drill = {
    mining_speed = 0.25 / second,
    pollution = 12 / minute,
    mining_area_side = 2,
    fuel_type = burnable_fuel,
    max_consumption = 150 * kW,
  },
  electric_mining_drill = {
    mining_speed = 0.5 / second,
    pollution = 10 / minute,
    mining_area_side = 5,
    fuel_type = electricity,
    max_consumption = 90 * kW,
  },
  stone_furnace = {
    crafting_speed = 1,
    pollution = 2 / minute,
    max_consumption = 90 * kW,
    fuel_type = burnable_fuel,
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
  assembling_machine_1 = {
    min_consumption = 2.5 * kW,
    max_consumption = 77.5 * kW,
    pollution = 4 / minute,
    crafting_speed = 0.5,
  },
}

biters = {
  small_biter = {
    health = 15,
    spawn_pollution = 3.2,
    resistances = {
      physical = { 0, 0 }
    }
  },
  medium_biter = {
    health = 75,
    spawn_pollution = 20,
    resistances = {
      physical = { 4, 0.1 }
    }
  },
  big_biter = {
    health = 375,
    spawn_pollution = 80,
    resistances = {
      physical = { 8, 0.1 }
    }
  },
}

function burner_drill_pollution(amount_to_drill)
  if amount_to_drill < 1e-5 then
    return 0
  end
  local drill = stuff.burner_mining_drill
  local drill_time = amount_to_drill / drill.mining_speed
  return
      drill_time * drill.pollution +
      burnable_fuel_pollution(drill_time * drill.max_consumption)
end

function electricity_pollution(joules)
  if joules < 1e-5 then
    return 0
  end
  local engine = stuff.steam_engine
  local steam_engine_time = joules / engine.max_output
  local need_steam = steam_engine_time * engine.consumes_steam
  local boiler = stuff.boiler
  local need_water = need_steam / boiler.generates_steam * boiler.consumes_water
  local boiler_time = need_water / boiler.consumes_water
  return
      boiler_time * boiler.pollution +
      burnable_fuel_pollution(boiler_time * boiler.max_consumption)
end

function electric_drill_pollution(amount_to_drill)
  if amount_to_drill < 1e-5 then
    return 0
  end
  local drill = stuff.electric_mining_drill
  local drill_time = amount_to_drill / drill.mining_speed
  return
      drill.pollution * drill_time +
      electricity_pollution(drill.max_consumption * drill_time)
end

function burnable_fuel_pollution(joules)
  local coal_used = joules / stuff.coal.fuel_value
  return drill_pollution(coal_used)
end

function smelt_pollution(recipe, items_to_smelt)
  local furnace = stuff.stone_furnace
  local time = items_to_smelt / furnace.crafting_speed * recipe.crafting_time
  return
      burnable_fuel_pollution(time * furnace.max_consumption) +
      time * furnace.pollution
end

-- print(drill_pollution(1))
-- print(smelt_pollution(recipes.iron_plate, 1))

function craft_pollution_without_making_ingredients(recipe, items_to_craft)
  local assembler = stuff.assembling_machine_1
  local time = recipe.crafting_time * items_to_craft / assembler.crafting_speed
  return
      time * assembler.pollution +
      electricity_pollution(time * assembler.max_consumption)
end

function iron_to_kill_pollution(pollution)
  if pollution < 1e-5 then
    return 0
  end
  -- biter = biters.small_biter
  -- biter = biters.medium_biter
  local biter = biters.big_biter
  local biters_spawned = pollution / biter.spawn_pollution
  return iron_to_kill_biters(biters_spawned)
end

function iron_to_kill_biters(amount)
  if amount < 1e-5 then
    return 0
  end
  local ammo = stuff.yellow_ammo
  local bullet_damage = math.max(1, ammo.damage - biter.resistances.physical[1]) * (1 - biter.resistances.physical[2])
  local magazine_damage = ammo.magazine_size * bullet_damage
  local total_health = biter.health * amount
  local ammo_used = total_health / magazine_damage
  print(ammo_used)
  local iron = ammo_used / 4
  local pollution =
      smelt_pollution(recipes.iron_plate, iron) +
      craft_pollution_without_making_ingredients(recipes.yellow_ammo, ammo_used)
  return iron + iron_to_kill_pollution(pollution)
end

furnace_used = stuff.stone_furnace
drill_pollution = burner_drill_pollution
biter = biters.medium_biter
-- drill_pollution = electric_drill_pollution

iron = 1000
defense_iron = iron_to_kill_pollution(smelt_pollution(recipes.iron_plate, iron))
print(defense_iron / iron * 100, "% of iron needs to go for making ammo")
