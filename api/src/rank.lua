-- Mesma regra de testes/oraculo.py.
-- Submissões da equipe ordenadas por (minuto, id).
-- ERRADO e TEMPO_EXCEDIDO contam antes do primeiro ACEITO.
-- ERRO_COMPILACAO não conta. Depois do acerto, o resto é ignorado.
-- Penalidade = minuto do acerto + 20 * erros anteriores.
-- Placar: mais resolvidos, menor penalidade, menor último acerto
-- (sem acerto ordena como 0, mas o JSON continua null), depois o id.

local function json_escape(s)
    local out = {}
    for i = 1, #s do
        local c = string.byte(s, i)
        if c == 34 then
            out[#out + 1] = '\\"'
        elseif c == 92 then
            out[#out + 1] = '\\\\'
        elseif c == 8 then
            out[#out + 1] = '\\b'
        elseif c == 12 then
            out[#out + 1] = '\\f'
        elseif c == 10 then
            out[#out + 1] = '\\n'
        elseif c == 13 then
            out[#out + 1] = '\\r'
        elseif c == 9 then
            out[#out + 1] = '\\t'
        elseif c < 32 then
            out[#out + 1] = string.format('\\u%04x', c)
        else
            out[#out + 1] = string.sub(s, i, i)
        end
    end
    return table.concat(out)
end

local function json_bool(valor)
    if valor then
        return 'true'
    end
    return 'false'
end

local function json_int_or_null(n)
    if n == nil then
        return 'null'
    end
    return string.format('%d', n)
end

local function encode(id, problema, minuto, veredito)
    return string.format(
        '%d\n%s%d\n%s%d\n%s%d\n%s',
        #id, id, #problema, problema, #minuto, minuto, #veredito, veredito
    )
end

local function decode(s)
    local p = 1
    local function take()
        local nl = string.find(s, '\n', p, true)
        local len = tonumber(string.sub(s, p, nl - 1))
        local inicio = nl + 1
        local v = string.sub(s, inicio, inicio + len - 1)
        p = inicio + len
        return v
    end
    return take(), take(), take(), take()
end

local function pack_resumo(nome, resolvidos, penalidade, ultimo)
    local u = ''
    if ultimo ~= nil then
        u = string.format('%d', ultimo)
    end
    return string.format('%d\n%s%d\n%d\n%s', #nome, nome, resolvidos, penalidade, u)
end

local function unpack_resumo(s)
    local nl = string.find(s, '\n', 1, true)
    local len = tonumber(string.sub(s, 1, nl - 1))
    local nome = string.sub(s, nl + 1, nl + len)
    local p = nl + 1 + len
    local nl2 = string.find(s, '\n', p, true)
    local resolvidos = tonumber(string.sub(s, p, nl2 - 1))
    p = nl2 + 1
    local nl3 = string.find(s, '\n', p, true)
    local penalidade = tonumber(string.sub(s, p, nl3 - 1))
    local ultimo = string.sub(s, nl3 + 1)
    if ultimo == '' then
        ultimo = nil
    else
        ultimo = tonumber(ultimo)
    end
    return nome, resolvidos, penalidade, ultimo
end

local function antes_sub(a, b)
    if a.minuto ~= b.minuto then
        return a.minuto < b.minuto
    end
    return a.id < b.id
end

local function antes_linha(a, b)
    if a.resolvidos ~= b.resolvidos then
        return a.resolvidos > b.resolvidos
    end
    if a.penalidade ~= b.penalidade then
        return a.penalidade < b.penalidade
    end
    local ua = a.ultimo or 0
    local ub = b.ultimo or 0
    if ua ~= ub then
        return ua < ub
    end
    return a.id < b.id
end

local function rebuild()
    local flat = redis.call('HGETALL', 'resumo')
    local rows = {}
    for i = 1, #flat, 2 do
        local nome, resolvidos, penalidade, ultimo = unpack_resumo(flat[i + 1])
        rows[#rows + 1] = {
            id = flat[i],
            nome = nome,
            resolvidos = resolvidos,
            penalidade = penalidade,
            ultimo = ultimo,
        }
    end
    table.sort(rows, antes_linha)
    local partes = {}
    for i = 1, #rows do
        local row = rows[i]
        partes[#partes + 1] = string.format(
            '{"posicao":%d,"equipe_id":"%s","nome":"%s","resolvidos":%d,"penalidade":%d,"ultimo_acerto":%s}',
            i,
            json_escape(row.id),
            json_escape(row.nome),
            row.resolvidos,
            row.penalidade,
            json_int_or_null(row.ultimo)
        )
    end
    redis.call('SET', 'placar', '[' .. table.concat(partes, ',') .. ']')
end

local function recompute(equipe_id, nome)
    local lista = redis.call('LRANGE', 'subs:' .. equipe_id, 0, -1)
    local subs = {}
    for i = 1, #lista do
        local sid, problema, minuto, veredito = decode(lista[i])
        subs[#subs + 1] = {
            id = sid,
            problema = problema,
            minuto = tonumber(minuto),
            veredito = veredito,
        }
    end
    table.sort(subs, antes_sub)

    local por = {}
    local ordem = {}
    for i = 1, #subs do
        local sub = subs[i]
        local p = por[sub.problema]
        if p == nil then
            p = { resolvido = false, tentativas = 0, minuto = nil, penalidade = 0 }
            por[sub.problema] = p
            ordem[#ordem + 1] = sub.problema
        end
        if not p.resolvido then
            if sub.veredito == 'ACEITO' then
                p.resolvido = true
                p.minuto = sub.minuto
                p.penalidade = sub.minuto + 20 * p.tentativas
            elseif sub.veredito == 'ERRADO' or sub.veredito == 'TEMPO_EXCEDIDO' then
                p.tentativas = p.tentativas + 1
            end
        end
    end
    table.sort(ordem)

    local resolvidos = 0
    local penalidade = 0
    local ultimo = nil
    local problemas = {}
    for i = 1, #ordem do
        local nome_problema = ordem[i]
        local p = por[nome_problema]
        if p.resolvido then
            resolvidos = resolvidos + 1
            penalidade = penalidade + p.penalidade
            if ultimo == nil or p.minuto > ultimo then
                ultimo = p.minuto
            end
        end
        problemas[#problemas + 1] = string.format(
            '{"problema":"%s","resolvido":%s,"tentativas_erradas":%d,"minuto_acerto":%s,"penalidade":%d}',
            json_escape(nome_problema),
            json_bool(p.resolvido),
            p.tentativas,
            json_int_or_null(p.minuto),
            p.penalidade
        )
    end

    local detalhe = string.format(
        '{"equipe_id":"%s","nome":"%s","resolvidos":%d,"penalidade":%d,"ultimo_acerto":%s,"problemas":[%s]}',
        json_escape(equipe_id),
        json_escape(nome),
        resolvidos,
        penalidade,
        json_int_or_null(ultimo),
        table.concat(problemas, ',')
    )
    redis.call('SET', 'detalhe:' .. equipe_id, detalhe)
    redis.call('HSET', 'resumo', equipe_id, pack_resumo(nome, resolvidos, penalidade, ultimo))
    rebuild()
end
