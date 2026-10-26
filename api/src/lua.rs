use std::sync::LazyLock;

use redis::Script;

const FUNCOES: &str = include_str!("rank.lua");

fn script(corpo: &str) -> Script {
    Script::new(&format!("{FUNCOES}\n{corpo}"))
}

pub static EQUIPE: LazyLock<Script> = LazyLock::new(|| {
    script(
        r#"
local id = ARGV[1]
local nome = ARGV[2]
if redis.call('EXISTS', 'equipe:' .. id) == 1 then
    return 409
end
redis.call('SET', 'equipe:' .. id, nome)
recompute(id, nome)
return 201
"#,
    )
});

pub static SUBMISSAO: LazyLock<Script> = LazyLock::new(|| {
    script(
        r#"
local id = ARGV[1]
local equipe = ARGV[2]
local problema = ARGV[3]
local minuto = ARGV[4]
local veredito = ARGV[5]
if redis.call('EXISTS', 'sub:' .. id) == 1 then
    return 200
end
if redis.call('EXISTS', 'equipe:' .. equipe) == 0 then
    return 404
end
local encoded = encode(id, problema, minuto, veredito)
redis.call('RPUSH', 'subs:' .. equipe, encoded)
local idx = redis.call('LLEN', 'subs:' .. equipe) - 1
redis.call('HSET', 'sub:' .. id, 'equipe', equipe, 'idx', idx, 'problema', problema, 'minuto', minuto, 'veredito', veredito)
local nome = redis.call('GET', 'equipe:' .. equipe)
recompute(equipe, nome)
return 201
"#,
    )
});

pub static REJULGAR: LazyLock<Script> = LazyLock::new(|| {
    script(
        r#"
local id = ARGV[1]
local veredito = ARGV[2]
if redis.call('EXISTS', 'sub:' .. id) == 0 then
    return 404
end
local h = redis.call('HMGET', 'sub:' .. id, 'equipe', 'idx', 'problema', 'minuto')
local equipe = h[1]
local idx = h[2]
local problema = h[3]
local minuto = h[4]
redis.call('HSET', 'sub:' .. id, 'veredito', veredito)
redis.call('LSET', 'subs:' .. equipe, idx, encode(id, problema, minuto, veredito))
local nome = redis.call('GET', 'equipe:' .. equipe)
recompute(equipe, nome)
return 200
"#,
    )
});
