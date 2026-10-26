# Rinha do Placar

API do placar de uma maratona no estilo ICPC, para a [Rinha do Placar](https://github.com/EduardoMilani8/rinha-do-placar).

## Tecnologias

- Rust (axum e tokio), duas instâncias iguais: `api01` e `api02`
- Redis 7 como estado compartilhado
- nginx na porta 9999, com failover se uma instância cair

As APIs não guardam o placar na própria memória. Cada escrita válida roda um script Lua no Redis: grava a submissão, recalcula só a equipe afetada e regrava o JSON do placar inteiro. `GET /placar` lê esse JSON pronto. Por isso as duas instâncias enxergam a mesma escrita, e uma leitura no meio de uma rajada continua sendo um ranking completo.

O Redis sobe sem AOF e sem RDB. A queda de uma API não apaga o que já foi confirmado. A queda do Redis não precisa ser suportada.

Limites declarados: nginx 0,1 CPU / 32 MB, cada API 0,3 CPU / 128 MB, Redis 0,8 CPU / 2712 MB. Total 1,5 CPU e 3000 MB.

## Como rodar

```bash
docker compose up --build -d
```

`GET http://localhost:9999/saude` responde 200 quando o Redis está acessível.

Os testes públicos ficam no repositório da organização:

```bash
git clone https://github.com/EduardoMilani8/rinha-do-placar /tmp/rinha-do-placar
python3 /tmp/rinha-do-placar/testes/executar.py corretude
python3 /tmp/rinha-do-placar/testes/executar.py carga --seed 42
```

## Uso de IA

A solução foi escrita com o assistente do Cursor (Grok). O assistente leu a especificação, a regra de ranking e o `testes/oraculo.py` públicos, propôs a arquitetura (APIs sem estado, escrita atômica no Redis, placar pré-montado) e redigiu o código Rust e o script Lua. A regra implementada no Lua é a do oráculo: ordem por minuto e id, penalidade de 20 por erro antes do primeiro acerto, erro de compilação ignorado, rejulgamento recalculando o passado.
