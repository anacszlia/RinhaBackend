use serde_json::{Map, Number, Value};

const VEREDITOS: &[&str] = &["ACEITO", "ERRADO", "ERRO_COMPILACAO", "TEMPO_EXCEDIDO"];

pub struct Equipe {
    pub id: String,
    pub nome: String,
}

pub struct Submissao {
    pub id: String,
    pub equipe_id: String,
    pub problema: String,
    pub minuto: i64,
    pub veredito: String,
}

pub fn equipe(corpo: &[u8]) -> Result<Equipe, ()> {
    let mapa = objeto(corpo)?;
    Ok(Equipe {
        id: texto(&mapa, "id")?,
        nome: texto(&mapa, "nome")?,
    })
}

pub fn submissao(corpo: &[u8]) -> Result<Submissao, ()> {
    let mapa = objeto(corpo)?;
    Ok(Submissao {
        id: texto(&mapa, "id")?,
        equipe_id: texto(&mapa, "equipe_id")?,
        problema: problema(&mapa)?,
        minuto: minuto(&mapa)?,
        veredito: veredito_em(&mapa, "veredito")?,
    })
}

pub fn veredito(corpo: &[u8]) -> Result<String, ()> {
    let mapa = objeto(corpo)?;
    veredito_em(&mapa, "veredito")
}

fn objeto(corpo: &[u8]) -> Result<Map<String, Value>, ()> {
    if corpo.is_empty() {
        return Err(());
    }
    match serde_json::from_slice(corpo) {
        Ok(Value::Object(mapa)) => Ok(mapa),
        _ => Err(()),
    }
}

fn texto(mapa: &Map<String, Value>, chave: &str) -> Result<String, ()> {
    match mapa.get(chave) {
        Some(Value::String(s)) if (1..=64).contains(&s.chars().count()) => Ok(s.clone()),
        _ => Err(()),
    }
}

fn problema(mapa: &Map<String, Value>) -> Result<String, ()> {
    match mapa.get("problema") {
        Some(Value::String(s)) if problema_ok(s) => Ok(s.clone()),
        _ => Err(()),
    }
}

fn problema_ok(s: &str) -> bool {
    let bytes = s.as_bytes();
    matches!(bytes.len(), 1 | 2) && bytes.iter().all(|b| b.is_ascii_uppercase())
}

fn minuto(mapa: &Map<String, Value>) -> Result<i64, ()> {
    match mapa.get("minuto") {
        Some(Value::Number(n)) => inteiro_0_a_300(n),
        _ => Err(()),
    }
}

fn inteiro_0_a_300(n: &Number) -> Result<i64, ()> {
    let texto = n.to_string();
    if texto.contains(['.', 'e', 'E', '+']) {
        return Err(());
    }
    let i = n.as_i64().ok_or(())?;
    if (0..=300).contains(&i) {
        Ok(i)
    } else {
        Err(())
    }
}

fn veredito_em(mapa: &Map<String, Value>, chave: &str) -> Result<String, ()> {
    match mapa.get(chave) {
        Some(Value::String(s)) if VEREDITOS.contains(&s.as_str()) => Ok(s.clone()),
        _ => Err(()),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn equipe_valida_e_limites() {
        assert!(equipe(br#"{"id":"eq-a","nome":"Equipe A","extra":1}"#).is_ok());
        assert!(equipe(br#"{"id":"","nome":"x"}"#).is_err());
        assert!(equipe(br#"{"id":"eq-x"}"#).is_err());
        assert!(equipe(br#"{"id":"eq","nome":""}"#).is_err());
        let longo = "a".repeat(65);
        let corpo = format!(r#"{{"id":"eq","nome":"{longo}"}}"#);
        assert!(equipe(corpo.as_bytes()).is_err());
        let acentos = "á".repeat(64);
        let corpo = format!(r#"{{"id":"eq","nome":"{acentos}"}}"#);
        assert!(equipe(corpo.as_bytes()).is_ok());
    }

    #[test]
    fn corpo_invalido() {
        assert!(equipe(b"").is_err());
        assert!(equipe(b"null").is_err());
        assert!(equipe(b"[]").is_err());
        assert!(equipe(b"true").is_err());
        assert!(equipe(b"{").is_err());
        assert!(submissao(b"42").is_err());
    }

    #[test]
    fn submissao_minuto_e_veredito() {
        let ok = br#"{"id":"s1","equipe_id":"eq-a","problema":"AB","minuto":0,"veredito":"ACEITO"}"#;
        assert!(submissao(ok).is_ok());
        let ok = br#"{"id":"s1","equipe_id":"eq-a","problema":"Z","minuto":300,"veredito":"ERRO_COMPILACAO"}"#;
        assert!(submissao(ok).is_ok());

        let casos = [
            br#"{"id":"s1","equipe_id":"eq-a","problema":"a","minuto":1,"veredito":"ACEITO"}"#.as_slice(),
            br#"{"id":"s1","equipe_id":"eq-a","problema":"ABC","minuto":1,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":-1,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":301,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":10.5,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":10.0,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":"10","veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":true,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":false,"veredito":"ACEITO"}"#,
            br#"{"id":"s1","equipe_id":"eq-a","problema":"A","minuto":10,"veredito":"TALVEZ"}"#,
        ];
        for caso in casos {
            assert!(submissao(caso).is_err(), "aceitou {}", String::from_utf8_lossy(caso));
        }
    }

    #[test]
    fn patch_valida_so_o_veredito() {
        assert_eq!(veredito(br#"{"veredito":"ERRADO","outro":1}"#).unwrap(), "ERRADO");
        assert!(veredito(br#"{"veredito":"TALVEZ"}"#).is_err());
        assert!(veredito(br#"{}"#).is_err());
    }
}
