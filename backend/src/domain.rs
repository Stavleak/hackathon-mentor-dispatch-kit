use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    session_minutes: u16,
    mentors: Vec<Mentor>,
    requests: Vec<Request>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mentor {
    id: String,
    name: String,
    skills: Vec<String>,
    languages: Vec<String>,
    available_from: String,
    available_until: String,
    enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Queued,
    Assigned,
    InSession,
    Closed,
    Cancelled,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    team_code: String,
    skill: String,
    language: String,
    question: String,
    queued_at: String,
    owner: String,
    deadline: String,
    next_step: String,
    status: Status,
    mentor_id: Option<String>,
    assigned_at: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    closure_reason: String,
    manual_exception: String,
}

fn timestamp(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    if value.len() > 40 {
        return Err(format!("{field}: слишком длинная временная метка"));
    }
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| format!("{field}: нужна дата RFC 3339 с часовым поясом"))
}

fn text(value: &str, field: &str, max: usize, required: bool) -> Result<(), String> {
    if (required && value.trim().is_empty()) || value.chars().count() > max {
        return Err(format!("{field}: заполните поле, максимум {max} символов"));
    }
    if value.chars().any(|character| character.is_control() && character != '\n') {
        return Err(format!("{field}: управляющие символы недопустимы"));
    }
    Ok(())
}

fn identifier(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(format!("{field}: используйте 1–64 латинских буквы, цифры, - или _"));
    }
    Ok(())
}

fn values(values: &[String], field: &str) -> Result<(), String> {
    if values.is_empty() || values.len() > 20 {
        return Err(format!("{field}: нужно от 1 до 20 значений"));
    }
    let mut unique = HashSet::new();
    for value in values {
        identifier(value, field)?;
        if !unique.insert(value) {
            return Err(format!("{field}: повторяющееся значение"));
        }
    }
    Ok(())
}

fn open(request: &Request) -> bool {
    request.status != Status::Closed && request.status != Status::Cancelled
}

fn reserved(request: &Request) -> bool {
    request.status == Status::Assigned || request.status == Status::InSession
}

fn compatible(mentor: &Mentor, request: &Request) -> bool {
    mentor.skills.contains(&request.skill) && mentor.languages.contains(&request.language)
}

fn order_key(request: &Request) -> (DateTime<Utc>, &str) {
    // Called only after typed validation has accepted every timestamp.
    (timestamp(&request.queued_at, "queued_at").expect("validated timestamp"), &request.id)
}

pub fn default_data() -> Value {
    json!({"session_minutes": 15, "mentors": [], "requests": []})
}

pub fn validate(value: &Value) -> Result<(), String> {
    let data: Data = serde_json::from_value(value.clone())
        .map_err(|error| format!("Неверная структура данных: {error}"))?;
    validate_typed(&data)
}

pub fn validate_update(previous: &Value, next: &Value) -> Result<(), String> {
    validate(next)?;
    let before: Data = serde_json::from_value(previous.clone())
        .map_err(|error| format!("Прежнее состояние не читается: {error}"))?;
    let after: Data = serde_json::from_value(next.clone())
        .map_err(|error| format!("Новое состояние не читается: {error}"))?;
    for old in &before.requests {
        let new = after.requests.iter().find(|request| request.id == old.id)
            .ok_or_else(|| format!("{}: запрос нельзя удалить; используйте явный отзыв", old.id))?;
        if !open(old) && old != new {
            return Err(format!("{}: завершённую запись нельзя переписывать; создайте новый номер", old.id));
        }
        if old.team_code != new.team_code || old.queued_at != new.queued_at {
            return Err(format!("{}: код команды и исходное время очереди неизменны", old.id));
        }
        let allowed = match old.status {
            Status::Queued => matches!(new.status, Status::Queued | Status::Assigned | Status::Closed | Status::Cancelled),
            Status::Assigned => matches!(new.status, Status::Assigned | Status::InSession | Status::Queued | Status::Cancelled),
            Status::InSession => matches!(new.status, Status::InSession | Status::Closed | Status::Cancelled),
            Status::Closed => new.status == Status::Closed,
            Status::Cancelled => new.status == Status::Cancelled,
        };
        if !allowed {
            return Err(format!("{}: недопустимый переход состояния", old.id));
        }
        if reserved(old) && new.status != Status::Queued
            && (old.mentor_id != new.mentor_id || old.assigned_at != new.assigned_at
                || old.skill != new.skill || old.language != new.language || old.manual_exception != new.manual_exception)
        {
            return Err(format!("{}: сначала верните назначение в очередь; факты назначения менять нельзя", old.id));
        }
        if old.status == Status::InSession && old.started_at != new.started_at {
            return Err(format!("{}: время начавшейся сессии неизменно", old.id));
        }
    }
    for request in &after.requests {
        let old = before.requests.iter().find(|old| old.id == request.id);
        if old.is_none() && request.status != Status::Queued {
            return Err(format!("{}: новый запрос должен поступить в очередь", request.id));
        }
        let changed_assignment = request.status == Status::Assigned && old.is_some_and(|old| old.status == Status::Queued);
        let changed_start = request.status == Status::InSession && old.is_some_and(|old| old.status == Status::Assigned);
        let changed_end = !open(request) && old.is_some_and(|old| old.status == Status::InSession);
        for (changed, time, field) in [
            (changed_assignment, request.assigned_at.as_deref(), "assigned_at"),
            (changed_start, request.started_at.as_deref(), "started_at"),
            (changed_end, request.ended_at.as_deref(), "ended_at"),
        ] {
            if changed {
                let actual = timestamp(time.ok_or_else(|| format!("{}: {field} отсутствует", request.id))?, field)?;
                if (actual - Utc::now()).num_seconds().abs() > 120 {
                    return Err(format!("{}: {field} должно отражать действие сейчас; проверьте часы устройства", request.id));
                }
            }
        }
    }
    Ok(())
}

fn validate_typed(data: &Data) -> Result<(), String> {
    if !(5..=60).contains(&data.session_minutes) {
        return Err("Длительность сессии должна быть от 5 до 60 минут".into());
    }
    if data.mentors.len() > 100 || data.requests.len() > 2000 {
        return Err("Лимит рабочей среды: 100 менторов и 2000 запросов".into());
    }
    let mut mentors = HashMap::new();
    for mentor in &data.mentors {
        identifier(&mentor.id, "mentor.id")?;
        text(&mentor.name, "mentor.name", 120, true)?;
        values(&mentor.skills, "mentor.skills")?;
        values(&mentor.languages, "mentor.languages")?;
        if mentor.languages.iter().any(|language| !matches!(language.as_str(), "ru" | "kk" | "en")) {
            return Err("Языки ментора: ru, kk или en".into());
        }
        let start = timestamp(&mentor.available_from, "mentor.available_from")?;
        let end = timestamp(&mentor.available_until, "mentor.available_until")?;
        if end <= start {
            return Err(format!("{}: конец доступности должен быть после начала", mentor.id));
        }
        if mentors.insert(mentor.id.as_str(), mentor).is_some() {
            return Err(format!("Повторяющийся ID ментора: {}", mentor.id));
        }
    }
    let mut ids = HashSet::new();
    let mut teams = HashSet::new();
    let mut occupied = HashSet::new();
    for request in &data.requests {
        identifier(&request.id, "request.id")?;
        identifier(&request.team_code, "request.team_code")?;
        identifier(&request.skill, "request.skill")?;
        text(&request.question, "request.question", 1000, true)?;
        text(&request.owner, "request.owner", 120, true)?;
        text(&request.next_step, "request.next_step", 500, open(request))?;
        text(&request.manual_exception, "request.manual_exception", 500, false)?;
        text(&request.closure_reason, "request.closure_reason", 40, false)?;
        if !matches!(request.language.as_str(), "ru" | "kk" | "en") {
            return Err("Язык запроса: ru, kk или en".into());
        }
        let queued = timestamp(&request.queued_at, "request.queued_at")?;
        let deadline = timestamp(&request.deadline, "request.deadline")?;
        if deadline < queued {
            return Err(format!("{}: срок действия раньше поступления в очередь", request.id));
        }
        if !ids.insert(&request.id) {
            return Err(format!("Повторяющийся ID запроса: {}", request.id));
        }
        if open(request) && !teams.insert(&request.team_code) {
            return Err(format!("{}: у команды уже есть открытый запрос", request.team_code));
        }
        let assigned = request.assigned_at.as_deref().map(|time| timestamp(time, "assigned_at")).transpose()?;
        let started = request.started_at.as_deref().map(|time| timestamp(time, "started_at")).transpose()?;
        let ended = request.ended_at.as_deref().map(|time| timestamp(time, "ended_at")).transpose()?;
        if assigned.is_some_and(|time| time < queued)
            || started.is_some_and(|time| assigned.is_none_or(|assigned| time < assigned))
            || ended.is_some_and(|time| started.is_none_or(|started| time < started))
        {
            return Err(format!("{}: времена назначения, начала и конца несогласованы", request.id));
        }
        if request.status == Status::Queued
            && (request.mentor_id.is_some() || assigned.is_some() || started.is_some() || ended.is_some() || !request.closure_reason.is_empty())
        {
            return Err(format!("{}: ожидающий запрос не должен содержать назначение или итог", request.id));
        }
        if open(request) && !request.closure_reason.is_empty() {
            return Err(format!("{}: причина закрытия есть у открытого запроса", request.id));
        }
        if request.status == Status::Assigned && (assigned.is_none() || started.is_some() || ended.is_some()) {
            return Err(format!("{}: назначение требует assigned_at без начала и конца", request.id));
        }
        if request.status == Status::InSession && (assigned.is_none() || started.is_none() || ended.is_some()) {
            return Err(format!("{}: текущая сессия требует назначение и начало без конца", request.id));
        }
        if request.status == Status::Closed {
            if !matches!(request.closure_reason.as_str(), "answered" | "action_agreed" | "unavailable") {
                return Err(format!("{}: укажите допустимый итог закрытия", request.id));
            }
            if started.is_some() && ended.is_none() {
                return Err(format!("{}: завершённая сессия требует ended_at", request.id));
            }
            if request.closure_reason != "unavailable" && started.is_none() {
                return Err(format!("{}: ответ или действие фиксируют после начала сессии", request.id));
            }
        }
        if request.status == Status::Cancelled
            && (request.closure_reason != "withdrawn" || (started.is_some() && ended.is_none()))
        {
            return Err(format!("{}: отмена требует withdrawn и конец начатой сессии", request.id));
        }
        if let Some(mentor_id) = &request.mentor_id {
            let mentor = mentors.get(mentor_id.as_str()).ok_or_else(|| format!("{}: неизвестный ментор", request.id))?;
            if assigned.is_none() {
                return Err(format!("{}: ссылка на ментора требует времени назначения", request.id));
            }
            if reserved(request) {
                if !mentor.enabled || !compatible(mentor, request) {
                    return Err(format!("{}: ментор не подтверждён или не подходит по теме и языку", request.id));
                }
                if !occupied.insert(mentor_id) {
                    return Err(format!("{mentor_id}: два активных назначения недопустимы"));
                }
                let assignment = assigned.expect("checked above");
                let from = timestamp(&mentor.available_from, "available_from")?;
                let until = timestamp(&mentor.available_until, "available_until")?;
                let session_start = started.unwrap_or(assignment);
                if assignment < from || session_start + Duration::minutes(i64::from(data.session_minutes)) > until {
                    return Err(format!("{}: сессия не помещается в подтверждённый интервал ментора", request.id));
                }
                if request.manual_exception.trim().is_empty()
                    && data.requests.iter().any(|earlier| earlier.status == Status::Queued
                        && compatible(mentor, earlier)
                        && timestamp(&earlier.queued_at, "queued_at").is_ok_and(|time| (time, earlier.id.as_str()) < (queued, request.id.as_str()) && time <= assignment))
                {
                    return Err(format!("{}: пропущен более ранний подходящий запрос; нужна причина исключения", request.id));
                }
            }
        } else if reserved(request) || assigned.is_some() || started.is_some() {
            return Err(format!("{}: назначение и сессия требуют ссылку на ментора", request.id));
        }
    }
    Ok(())
}

pub fn analyze(value: &Value) -> Value {
    analyze_at(value, Utc::now())
}

fn analyze_at(value: &Value, now: DateTime<Utc>) -> Value {
    let data: Data = match serde_json::from_value(value.clone()) {
        Ok(data) => data,
        Err(error) => return json!({"generated_at": now.to_rfc3339(), "open_count": 0, "overdue_ids": [], "recommendations": [], "unroutable_ids": [], "waiting_ids": [], "warnings": [format!("Неверная структура: {error}")]}),
    };
    if let Err(error) = validate_typed(&data) {
        return json!({"generated_at": now.to_rfc3339(), "open_count": 0, "overdue_ids": [], "recommendations": [], "unroutable_ids": [], "waiting_ids": [], "warnings": [error]});
    }
    let slot = Duration::minutes(i64::from(data.session_minutes));
    let mut occupied: HashSet<&str> = data.requests.iter().filter(|request| reserved(request)).filter_map(|request| request.mentor_id.as_deref()).collect();
    let mut queued: Vec<&Request> = data.requests.iter().filter(|request| request.status == Status::Queued).collect();
    queued.sort_by_key(|request| order_key(request));
    let mut recommendations = Vec::new();
    let mut unroutable = Vec::new();
    let mut waiting = Vec::new();
    let mut warnings = Vec::new();
    for request in queued {
        let ready = timestamp(&request.queued_at, "queued_at").expect("validated timestamp") <= now;
        let mut candidates: Vec<&Mentor> = data.mentors.iter().filter(|mentor| mentor.enabled
            && compatible(mentor, request)
            && !occupied.contains(mentor.id.as_str())
            && timestamp(&mentor.available_from, "available_from").expect("validated timestamp") <= now
            && timestamp(&mentor.available_until, "available_until").expect("validated timestamp") >= now + slot
            // A recommendation must be actionable by itself, not depend on another planned assignment.
            && !data.requests.iter().any(|earlier| earlier.status == Status::Queued && compatible(mentor, earlier)
                && order_key(earlier) < order_key(request)
                && timestamp(&earlier.queued_at, "queued_at").expect("validated timestamp") <= now)).collect();
        candidates.sort_by_key(|mentor| (
            data.requests.iter().filter(|request| request.mentor_id.as_deref() == Some(mentor.id.as_str()) && request.started_at.is_some()).count(),
            mentor.id.as_str(),
        ));
        if let Some(mentor) = candidates.first().filter(|_| ready) {
            recommendations.push(json!({"request_id": request.id, "mentor_id": mentor.id}));
            occupied.insert(&mentor.id);
        } else if data.mentors.iter().any(|mentor| mentor.enabled && compatible(mentor, request)
            && timestamp(&mentor.available_until, "available_until").expect("validated timestamp") >= now + slot)
        {
            waiting.push(&request.id);
        } else {
            unroutable.push(&request.id);
        }
    }
    for request in data.requests.iter().filter(|request| reserved(request)) {
        if timestamp(&request.deadline, "deadline").expect("validated timestamp") < now {
            warnings.push(format!("{}: срок назначения или сессии прошёл; проверьте контакт и обновите следующий шаг", request.id));
        }
        if !request.manual_exception.is_empty() {
            warnings.push(format!("{}: ручное исключение из очереди — {}", request.id, request.manual_exception));
        }
    }
    json!({
        "generated_at": now.to_rfc3339(),
        "open_count": data.requests.iter().filter(|request| open(request)).count(),
        "overdue_ids": data.requests.iter().filter(|request| open(request) && timestamp(&request.deadline, "deadline").expect("validated timestamp") < now).map(|request| &request.id).collect::<Vec<_>>(),
        "recommendations": recommendations,
        "unroutable_ids": unroutable,
        "waiting_ids": waiting,
        "warnings": warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        json!({"session_minutes": 15, "mentors": [{"id":"M-1", "name":"Учебный ментор", "skills":["tech"], "languages":["ru"], "available_from":"2026-10-10T09:00:00Z", "available_until":"2026-10-10T12:00:00Z", "enabled":true}], "requests":[
            {"id":"Q-1","team_code":"T-1","skill":"tech","language":"ru","question":"Первый учебный вопрос","queued_at":"2026-10-10T09:01:00Z","owner":"D-1","deadline":"2026-10-10T10:05:00Z","next_step":"Назначить ментора","status":"queued","mentor_id":null,"assigned_at":null,"started_at":null,"ended_at":null,"closure_reason":"","manual_exception":""},
            {"id":"Q-2","team_code":"T-2","skill":"tech","language":"ru","question":"Второй учебный вопрос","queued_at":"2026-10-10T09:02:00Z","owner":"D-1","deadline":"2026-10-10T10:05:00Z","next_step":"Назначить ментора","status":"queued","mentor_id":null,"assigned_at":null,"started_at":null,"ended_at":null,"closure_reason":"","manual_exception":""}
        ]})
    }

    fn assign(value: &mut Value, index: usize) {
        value["requests"][index]["status"] = json!("assigned");
        value["requests"][index]["mentor_id"] = json!("M-1");
        value["requests"][index]["assigned_at"] = json!("2026-10-10T10:00:00Z");
    }

    #[test]
    fn empty_defaults_and_strict_schema() {
        assert!(validate(&default_data()).is_ok());
        let mut value = default_data();
        value["unexpected"] = json!(true);
        assert!(validate(&value).is_err());
        value = fixture();
        value["requests"][0]["status"] = json!("unknown");
        assert!(validate(&value).is_err());
    }

    #[test]
    fn duplicates_and_dangling_references_are_rejected() {
        let mut value = fixture();
        value["requests"][1]["team_code"] = json!("T-1");
        assert!(validate(&value).unwrap_err().contains("открытый"));
        value = fixture();
        value["requests"][1]["id"] = json!("Q-1");
        assert!(validate(&value).is_err());
        value = fixture();
        assign(&mut value, 0);
        value["requests"][0]["mentor_id"] = json!("M-MISSING");
        assert!(validate(&value).unwrap_err().contains("неизвестный"));
    }

    #[test]
    fn fifo_exception_is_explicit_and_cannot_bypass_skills() {
        let mut value = fixture();
        assign(&mut value, 1);
        assert!(validate(&value).unwrap_err().contains("ранний"));
        value["requests"][1]["manual_exception"] = json!("Руководитель разрешил учебное исключение");
        assert!(validate(&value).is_ok());
        value["requests"][1]["skill"] = json!("hardware");
        assert!(validate(&value).unwrap_err().contains("не подходит"));
    }

    #[test]
    fn one_mentor_cannot_hold_two_assignments() {
        let mut value = fixture();
        assign(&mut value, 0);
        assign(&mut value, 1);
        assert!(validate(&value).unwrap_err().contains("два активных"));
    }

    #[test]
    fn analysis_recommends_oldest_and_exposes_missing_coverage() {
        let mut value = fixture();
        value["requests"][1]["skill"] = json!("hardware");
        let now = timestamp("2026-10-10T10:00:00Z", "test").unwrap();
        let analysis = analyze_at(&value, now);
        assert_eq!(analysis["recommendations"], json!([{"request_id":"Q-1","mentor_id":"M-1"}]));
        assert_eq!(analysis["unroutable_ids"], json!(["Q-2"]));
        assert!(analysis["waiting_ids"].as_array().unwrap().is_empty());
    }

    #[test]
    fn real_offsets_and_slot_capacity_determine_order() {
        let mut value = fixture();
        value["requests"][0]["queued_at"] = json!("2026-10-10T14:05:00+05:00");
        let now = timestamp("2026-10-10T10:00:00Z", "test").unwrap();
        assert_eq!(analyze_at(&value, now)["recommendations"][0]["request_id"], json!("Q-2"));
        value["mentors"][0]["available_until"] = json!("2026-10-10T10:10:00Z");
        assert!(analyze_at(&value, now)["recommendations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn invalid_lifecycle_times_and_empty_next_actions_are_rejected() {
        let mut value = fixture();
        value["requests"][0]["next_step"] = json!("");
        assert!(validate(&value).is_err());
        value = fixture();
        assign(&mut value, 0);
        value["requests"][0]["status"] = json!("in_session");
        value["requests"][0]["started_at"] = json!("2026-10-10T09:59:00Z");
        assert!(validate(&value).is_err());
        value["requests"][0]["started_at"] = json!("2026-10-10T10:00:00Z");
        assert!(validate(&value).is_ok());
    }

    #[test]
    fn updates_cannot_delete_reorder_or_rewrite_closed_requests() {
        let before = fixture();
        let mut after = before.clone();
        after["requests"].as_array_mut().unwrap().pop();
        assert!(validate_update(&before, &after).unwrap_err().contains("удалить"));
        after = before.clone();
        after["requests"][0]["queued_at"] = json!("2026-10-10T09:03:00Z");
        assert!(validate_update(&before, &after).unwrap_err().contains("неизменны"));
        let mut closed = before.clone();
        closed["requests"][0]["status"] = json!("closed");
        closed["requests"][0]["closure_reason"] = json!("unavailable");
        after = closed.clone();
        after["requests"][0]["question"] = json!("Переписанный итог");
        assert!(validate_update(&closed, &after).unwrap_err().contains("переписывать"));
    }

    #[test]
    fn new_records_and_lifecycle_skips_are_rejected() {
        let mut after = fixture();
        assign(&mut after, 0);
        assert!(validate_update(&default_data(), &after).unwrap_err().contains("новый запрос"));
        let before = fixture();
        after = before.clone();
        assign(&mut after, 0);
        after["requests"][0]["status"] = json!("in_session");
        after["requests"][0]["started_at"] = json!("2026-10-10T10:00:00Z");
        assert!(validate_update(&before, &after).unwrap_err().contains("переход"));
    }

    #[test]
    fn recommendations_do_not_depend_on_unsaved_planned_assignments() {
        let mut value = fixture();
        let mut second = value["mentors"][0].clone();
        second["id"] = json!("M-2");
        value["mentors"].as_array_mut().unwrap().push(second);
        let analysis = analyze_at(&value, timestamp("2026-10-10T10:00:00Z", "test").unwrap());
        assert_eq!(analysis["recommendations"].as_array().unwrap().len(), 1);
        assert_eq!(analysis["recommendations"][0]["request_id"], json!("Q-1"));
        assert_eq!(analysis["waiting_ids"], json!(["Q-2"]));
    }

    #[test]
    fn current_time_assignment_start_and_finish_are_accepted() {
        let now = Utc::now();
        let mut queued = fixture();
        queued["mentors"][0]["available_from"] = json!((now - Duration::hours(1)).to_rfc3339());
        queued["mentors"][0]["available_until"] = json!((now + Duration::hours(1)).to_rfc3339());
        for (index, request) in queued["requests"].as_array_mut().unwrap().iter_mut().enumerate() {
            request["queued_at"] = json!((now - Duration::minutes(2 - index as i64)).to_rfc3339());
            request["deadline"] = json!((now + Duration::minutes(5)).to_rfc3339());
        }
        let mut assigned = queued.clone();
        assign(&mut assigned, 0);
        assigned["requests"][0]["assigned_at"] = json!(now.to_rfc3339());
        assert!(validate_update(&queued, &assigned).is_ok());
        let mut started = assigned.clone();
        started["requests"][0]["status"] = json!("in_session");
        started["requests"][0]["started_at"] = json!(now.to_rfc3339());
        assert!(validate_update(&assigned, &started).is_ok());
        let mut finished = started.clone();
        finished["requests"][0]["status"] = json!("closed");
        finished["requests"][0]["ended_at"] = json!(now.to_rfc3339());
        finished["requests"][0]["closure_reason"] = json!("action_agreed");
        finished["requests"][0]["next_step"] = json!("Команда проверит согласованный пример самостоятельно");
        assert!(validate_update(&started, &finished).is_ok());
    }
}
