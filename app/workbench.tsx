"use client";

import { useState, type FormEvent } from "react";
import { useWorkbench } from "./use-workbench";
import { AppShell, Button, Field, Panel, Status } from "./ui";
import { demoData, type Analysis, type Data, type Mentor, type Request, type RequestStatus } from "./domain";

const statusLabels: Record<RequestStatus, string> = {
  queued: "В очереди", assigned: "Назначен", in_session: "Консультация идёт", closed: "Завершён", cancelled: "Отозван",
};

function inputTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const pad = (number: number) => String(number).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function displayTime(value: string): string {
  return new Date(value).toLocaleString("ru-RU", { dateStyle: "short", timeStyle: "short" });
}

function get(form: FormData, name: string): string {
  const value = form.get(name);
  return typeof value === "string" ? value.trim() : "";
}

function iso(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) throw new Error("Укажите корректную дату и время.");
  return date.toISOString();
}

function options(value: string): string[] {
  return value.split(",").map((part) => part.trim()).filter(Boolean);
}

function open(request: Request): boolean {
  return request.status !== "closed" && request.status !== "cancelled";
}

function mentorFits(mentor: Mentor, request: Request): boolean {
  return mentor.skills.includes(request.skill) && mentor.languages.includes(request.language);
}

type MentorFormProps = { mentor: Mentor | null; disabled: boolean; submit: (form: FormData) => Promise<void>; cancel: () => void };

function MentorForm({ mentor, disabled, submit, cancel }: MentorFormProps) {
  return (
    <form className="grid" onSubmit={(event: FormEvent<HTMLFormElement>) => { event.preventDefault(); void submit(new FormData(event.currentTarget)); }}>
      <Field label="Код ментора" name="id" defaultValue={mentor?.id ?? ""} placeholder="M-001" required maxLength={64} readOnly={Boolean(mentor)} />
      <Field label="Имя или рабочая роль" name="name" defaultValue={mentor?.name ?? ""} required maxLength={120} />
      <Field label="Темы через запятую" name="skills" defaultValue={mentor?.skills.join(", ") ?? ""} placeholder="tech, product, pitch" required />
      <Field label="Языки через запятую" name="languages" defaultValue={mentor?.languages.join(", ") ?? "ru"} placeholder="ru, kk, en" required />
      <Field label="Доступен с — время браузера" name="available_from" type="datetime-local" defaultValue={mentor ? inputTime(mentor.available_from) : ""} required />
      <Field label="Доступен до — время браузера" name="available_until" type="datetime-local" defaultValue={mentor ? inputTime(mentor.available_until) : ""} required />
      <Field label="Ментор подтвердил доступность" name="enabled" type="checkbox" defaultChecked={mentor?.enabled ?? false} />
      <div className="actions">
        <Button type="submit" disabled={disabled}>{mentor ? "Сохранить ментора" : "Добавить ментора"}</Button>
        {mentor && <Button type="button" variant="ghost" disabled={disabled} onClick={cancel}>Отменить редактирование</Button>}
      </div>
    </form>
  );
}

type RequestFormProps = { request: Request | null; disabled: boolean; submit: (form: FormData) => Promise<void>; cancel: () => void };

function RequestForm({ request, disabled, submit, cancel }: RequestFormProps) {
  const assigned = request?.status === "assigned" || request?.status === "in_session";
  return (
    <form className="grid" onSubmit={(event: FormEvent<HTMLFormElement>) => { event.preventDefault(); void submit(new FormData(event.currentTarget)); }}>
      <Field label="Номер запроса" name="id" defaultValue={request?.id ?? ""} placeholder="Q-001" required maxLength={64} readOnly={Boolean(request)} />
      <Field label="Внутренний код команды" name="team_code" defaultValue={request?.team_code ?? ""} placeholder="T-01" required maxLength={64} readOnly={Boolean(request)} />
      <Field label="Тема" name="skill" defaultValue={request?.skill ?? ""} placeholder="tech" required maxLength={64} readOnly={assigned} />
      <label className="field">
        <span>Язык</span>
        <select name="language" defaultValue={request?.language ?? "ru"} disabled={assigned} aria-label="Язык запроса">
          <option value="ru">Русский</option><option value="kk">Қазақша</option><option value="en">English</option>
        </select>
      </label>
      <label className="field"><span>Один безопасный вопрос</span><textarea name="question" required maxLength={1000} rows={3} defaultValue={request?.question ?? ""} /></label>
      <Field label="Готовность вопроса — время браузера" name="queued_at" type="datetime-local" defaultValue={request ? inputTime(request.queued_at) : ""} required readOnly={Boolean(request)} />
      <Field label="Текущий владелец записи" name="owner" defaultValue={request?.owner ?? ""} placeholder="Код диспетчера" required maxLength={120} />
      <Field label="Обновить статус до — время браузера" name="deadline" type="datetime-local" defaultValue={request ? inputTime(request.deadline) : ""} required />
      <Field label="Следующее действие владельца" name="next_step" defaultValue={request?.next_step ?? ""} required maxLength={500} />
      <div className="actions">
        <Button type="submit" disabled={disabled}>{request ? "Сохранить запрос" : "Принять запрос"}</Button>
        {request && <Button type="button" variant="ghost" disabled={disabled} onClick={cancel}>Отменить редактирование</Button>}
      </div>
    </form>
  );
}

type RequestCardProps = {
  request: Request; data: Data; analysis: Analysis | null; disabled: boolean;
  edit: () => void; patch: (next: Request) => Promise<void>;
};

function RequestCard({ request, data, analysis, disabled, edit, patch }: RequestCardProps) {
  const [mentorId, setMentorId] = useState("");
  const [exception, setException] = useState("");
  const [outcome, setOutcome] = useState("");
  const [reason, setReason] = useState("action_agreed");
  const recommendation = analysis?.recommendations.find((item) => item.request_id === request.id);
  const missing = analysis?.unroutable_ids.includes(request.id);
  const overdue = analysis?.overdue_ids.includes(request.id);
  const activeMentor = data.mentors.find((mentor) => mentor.id === request.mentor_id);
  const busy = new Set(data.requests.filter((item) => item.status === "assigned" || item.status === "in_session").map((item) => item.mentor_id));
  const now = analysis ? new Date(analysis.generated_at).getTime() : 0;
  const eligibleMentors = data.mentors.filter((mentor) => mentor.enabled && mentorFits(mentor, request) && !busy.has(mentor.id)
    && new Date(mentor.available_from).getTime() <= now && new Date(mentor.available_until).getTime() >= now + data.session_minutes * 60_000);

  async function assign(id: string, manual: string) {
    const assignedAt = new Date().toISOString();
    await patch({ ...request, status: "assigned", mentor_id: id, assigned_at: assignedAt, manual_exception: manual,
      deadline: new Date(Date.now() + 2 * 60_000).toISOString(), next_step: "Подтвердить контакт с ментором и начать встречу" });
  }

  return (
    <article className="queue-item stack" aria-label={`Запрос ${request.id}`}>
      <div className="actions">
        <strong>{request.id} · {request.team_code}</strong>
        <Status tone={request.status === "closed" ? "success" : "neutral"}>{statusLabels[request.status]}</Status>
        {overdue && <Status tone="danger">Просрочено обновление</Status>}
        {missing && <Status tone="warning">Нет подтверждённого специалиста</Status>}
      </div>
      <p>{request.question}</p>
      <p className="muted">{request.skill} · {request.language} · в очереди с {displayTime(request.queued_at)}</p>
      <p>Владелец: {request.owner}. Следующее действие: {request.next_step || "Запрос завершён"}. Срок: {displayTime(request.deadline)}.</p>
      {activeMentor && <p>Ментор: {activeMentor.name} ({activeMentor.id}).</p>}
      {request.manual_exception && <p>Причина ручного исключения: {request.manual_exception}</p>}
      {request.started_at && <p className="muted">Начало: {displayTime(request.started_at)}{request.ended_at ? ` · конец: ${displayTime(request.ended_at)}` : ""}.</p>}
      {request.closure_reason && <p>Исход: {request.closure_reason === "answered" ? "Ответ получен" : request.closure_reason === "action_agreed" ? "Согласовано действие команды" : request.closure_reason === "unavailable" ? "Доступная помощь отсутствует" : "Команда отозвала вопрос"}.</p>}

      {request.status === "queued" && (
        <div className="stack">
          {missing && <p>Проверьте резерв и сообщите команде доступный вариант. Ожидание остаётся открытым; новый срок задаётся через редактирование.</p>}
          {analysis?.waiting_ids.includes(request.id) && <p>Подходящее покрытие есть, но слот занят, ещё не начался или вопрос запланирован на позднее время.</p>}
          {recommendation && <Button type="button" disabled={disabled} onClick={() => void assign(recommendation.mentor_id, "")}>Назначить по очереди: {recommendation.mentor_id}</Button>}
          <form className="grid" onSubmit={(event) => { event.preventDefault(); if (mentorId && exception.trim()) void assign(mentorId, exception.trim()); }}>
            <label className="field"><span>Ручное назначение подходящего свободного ментора</span>
              <select value={mentorId} onChange={(event) => setMentorId(event.target.value)} required aria-label={`Ручное назначение ${request.id}`}>
                <option value="">Выберите ментора</option>
                {eligibleMentors.map((mentor) => <option key={mentor.id} value={mentor.id}>{mentor.name} · {mentor.id}</option>)}
              </select>
            </label>
            <Field label="Причина ручного исключения" value={exception} onChange={(event) => setException(event.target.value)} maxLength={500} required />
            <Button type="submit" variant="secondary" disabled={disabled || !mentorId || !exception.trim()}>Назначить с объяснением</Button>
          </form>
        </div>
      )}

      {request.status === "assigned" && <div className="actions">
        <Button type="button" disabled={disabled} onClick={() => void patch({ ...request, status: "in_session", owner: request.mentor_id ?? request.owner, started_at: new Date().toISOString(), deadline: new Date(Date.now() + data.session_minutes * 60_000).toISOString(), next_step: "Провести сессию и согласовать итог с командой" })}>Начать консультацию</Button>
        <Button type="button" variant="secondary" disabled={disabled} onClick={() => void patch({ ...request, status: "queued", mentor_id: null, assigned_at: null, started_at: null, ended_at: null, manual_exception: "", deadline: new Date(Date.now() + 5 * 60_000).toISOString(), next_step: "Подобрать другого ментора; прежняя позиция сохранена" })}>Вернуть в очередь</Button>
      </div>}

      {request.status === "in_session" && <form className="grid" onSubmit={(event) => {
        event.preventDefault(); if (!outcome.trim()) return;
        void patch({ ...request, status: "closed", ended_at: new Date().toISOString(), next_step: outcome.trim(), closure_reason: reason });
      }}>
        <Field label="Согласованный итог и действие команды" value={outcome} onChange={(event) => setOutcome(event.target.value)} required maxLength={500} />
        <label className="field"><span>Исход консультации</span><select value={reason} onChange={(event) => setReason(event.target.value)} aria-label={`Исход ${request.id}`}>
          <option value="action_agreed">Согласовано самостоятельное действие</option><option value="answered">Получен ответ</option>
        </select></label>
        <Button type="submit" disabled={disabled || !outcome.trim()}>Завершить сессию</Button>
      </form>}

      {missing && request.status === "queued" && <Field label="Сообщённый и согласованный исход без специалиста" value={outcome} onChange={(event) => setOutcome(event.target.value)} maxLength={500} />}
      {open(request) && <div className="actions">
        <Button type="button" variant="secondary" disabled={disabled} onClick={edit}>Изменить владельца, срок и контекст</Button>
        <Button type="button" variant="danger" disabled={disabled} onClick={() => {
          if (window.confirm(`Команда отозвала запрос ${request.id}?`)) void patch({ ...request, status: "cancelled", closure_reason: "withdrawn", ended_at: request.started_at ? new Date().toISOString() : null, next_step: "Команда отозвала запрос" });
        }}>Отозвать запрос</Button>
        {missing && <Button type="button" variant="secondary" disabled={disabled || !outcome.trim()} onClick={() => {
          if (window.confirm(`Команде сообщён итог по ${request.id} и она согласна закрыть вопрос без консультации?`)) void patch({ ...request, status: "closed", closure_reason: "unavailable", next_step: outcome.trim() });
        }}>Закрыть с согласованным исходом</Button>}
      </div>}
    </article>
  );
}

export default function Workbench() {
  const { data, analysis, revision, loading, saving, error, conflict, save, reload } = useWorkbench<Data, Analysis>();
  const [mentorEditId, setMentorEditId] = useState<string | null>(null);
  const [requestEditId, setRequestEditId] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const [showClosed, setShowClosed] = useState(false);
  const blocked = loading || saving || Boolean(conflict);
  const editingMentor = data?.mentors.find((mentor) => mentor.id === mentorEditId) ?? null;
  const editingRequest = data?.requests.find((request) => request.id === requestEditId) ?? null;

  async function persist(next: Data, message: string): Promise<boolean> {
    setNotice("");
    const success = await save(next);
    if (success) setNotice(message);
    return success;
  }

  async function submitMentor(form: FormData) {
    if (!data) return;
    try {
      const mentor: Mentor = { id: get(form, "id"), name: get(form, "name"), skills: options(get(form, "skills")), languages: options(get(form, "languages")), available_from: iso(get(form, "available_from")), available_until: iso(get(form, "available_until")), enabled: form.get("enabled") === "on" };
      const next = { ...data, mentors: editingMentor ? data.mentors.map((item) => item.id === editingMentor.id ? mentor : item) : [...data.mentors, mentor] };
      if (await persist(next, "Доступность ментора сохранена.")) setMentorEditId(null);
    } catch (failure) { setNotice(failure instanceof Error ? failure.message : "Проверьте поля ментора."); }
  }

  async function submitRequest(form: FormData) {
    if (!data) return;
    try {
      const fields = { id: get(form, "id"), team_code: get(form, "team_code"), skill: get(form, "skill"), language: get(form, "language") || editingRequest?.language || "ru", question: get(form, "question"), queued_at: editingRequest?.queued_at ?? iso(get(form, "queued_at")), owner: get(form, "owner"), deadline: iso(get(form, "deadline")), next_step: get(form, "next_step") };
      const request: Request = editingRequest ? { ...editingRequest, ...fields } : { ...fields, status: "queued", mentor_id: null, assigned_at: null, started_at: null, ended_at: null, closure_reason: "", manual_exception: "" };
      const next = { ...data, requests: editingRequest ? data.requests.map((item) => item.id === editingRequest.id ? request : item) : [...data.requests, request] };
      if (await persist(next, editingRequest ? "Запрос обновлён." : "Запрос принят в очередь.")) setRequestEditId(null);
    } catch (failure) { setNotice(failure instanceof Error ? failure.message : "Проверьте поля запроса."); }
  }

  async function patch(request: Request) {
    if (data) await persist({ ...data, requests: data.requests.map((item) => item.id === request.id ? request : item) }, `Запрос ${request.id} обновлён.`);
  }

  async function addDemo() {
    if (!data || !window.confirm("Добавить три вымышленных запроса и двух учебных менторов? Учебные записи будут помечены DEMO, существующие сохранятся.")) return;
    const now = Date.now();
    const mentors = demoData.mentors.map((mentor) => ({ ...mentor, skills: [...mentor.skills], languages: [...mentor.languages], available_from: new Date(now - 10 * 60_000).toISOString(), available_until: new Date(now + 120 * 60_000).toISOString() }));
    const requests = demoData.requests.map((request, index) => ({ ...request, queued_at: new Date(now - (5 - index) * 60_000).toISOString(), deadline: new Date(now + (5 + index) * 60_000).toISOString() }));
    await persist({ ...data, mentors: [...data.mentors, ...mentors], requests: [...data.requests, ...requests] }, "Добавлены вымышленные записи DEMO. Их можно обработать как учебную очередь.");
  }

  const hasDemo = data?.mentors.some((mentor) => demoData.mentors.some((example) => example.id === mentor.id)) || data?.requests.some((request) => demoData.requests.some((example) => example.id === request.id));
  const requests = data?.requests.filter((request) => showClosed || open(request)).sort((left, right) => new Date(left.queued_at).getTime() - new Date(right.queued_at).getTime() || left.id.localeCompare(right.id)) ?? [];

  return (
    <AppShell title="Очередь менторской помощи" description="Примите один вопрос команды, назначьте подходящего ментора и сохраните понятный следующий шаг." loading={loading} error={error}>
      {conflict && <Panel title="Очередь изменена в другой вкладке"><p>Перед сохранением загрузите актуальную версию и проверьте своё изменение. Несохранённые поля формы останутся на экране.</p><Button type="button" variant="secondary" onClick={() => void reload()} disabled={saving}>Загрузить актуальную очередь</Button></Panel>}
      {notice && <p role="status">{notice}</p>}
      {data && <>
        <Panel title="Рабочая смена">
          <div className="actions"><Status>{saving ? "Сохранение…" : `Версия ${revision}`}</Status><Status>Открыто: {analysis?.open_count ?? data.requests.filter(open).length}</Status><Status tone={analysis?.overdue_ids.length ? "danger" : "neutral"}>Просрочено: {analysis?.overdue_ids.length ?? 0}</Status></div>
          <p className="muted">Время форм отображается в часовом поясе браузера: {Intl.DateTimeFormat().resolvedOptions().timeZone}. Номер и код команды используют латиницу, цифры, дефис или подчёркивание. Контакты и секреты храните отдельно.</p>
          <form className="actions" onSubmit={(event) => { event.preventDefault(); const form = new FormData(event.currentTarget); void persist({ ...data, session_minutes: Number(get(form, "session_minutes")) }, "Длительность слота обновлена."); }}>
            <Field label="Длительность слота, минуты" type="number" name="session_minutes" min={5} max={60} required defaultValue={data.session_minutes} />
            <Button type="submit" disabled={blocked}>Сохранить длительность</Button>
            <Button type="button" variant="secondary" disabled={saving || loading} onClick={() => void reload()}>Обновить очередь и рекомендации</Button>
            <Button type="button" variant="secondary" disabled={blocked || Boolean(hasDemo)} onClick={() => void addDemo()}>Добавить учебный пример</Button>
          </form>
          {hasDemo && <p className="muted">В рабочей среде есть явно вымышленные записи DEMO.</p>}
          {analysis?.warnings.map((warning, index) => <p key={`${index}-${warning}`} role="status">{warning}</p>)}
          {analysis && <p className="muted">Рекомендации рассчитаны: {displayTime(analysis.generated_at)}. Перед назначением проверьте фактическую готовность специалиста.</p>}
        </Panel>

        <Panel title={editingMentor ? `Ментор ${editingMentor.id}` : "Добавить ментора"}>
          <MentorForm key={mentorEditId ?? "new-mentor"} mentor={editingMentor} disabled={blocked} submit={submitMentor} cancel={() => setMentorEditId(null)} />
          {data.mentors.length === 0 && <p>Менторов пока нет. Запишите подтверждённую тему, язык и полный интервал доступности.</p>}
          <ul className="list">{data.mentors.map((mentor) => <li key={mentor.id}>
            <p><strong>{mentor.name} · {mentor.id}</strong> — {mentor.skills.join(", ")}; {mentor.languages.join(", ")}. {displayTime(mentor.available_from)}–{displayTime(mentor.available_until)}.</p>
            <div className="actions"><Status tone={mentor.enabled ? "success" : "warning"}>{mentor.enabled ? "Доступность подтверждена" : "Не участвует в назначении"}</Status><Button type="button" variant="secondary" disabled={blocked} onClick={() => setMentorEditId(mentor.id)}>Редактировать ментора</Button></div>
          </li>)}</ul>
        </Panel>

        <Panel title={editingRequest ? `Редактировать ${editingRequest.id}` : "Принять запрос команды"}>
          <RequestForm key={requestEditId ?? "new-request"} request={editingRequest} disabled={blocked} submit={submitRequest} cancel={() => setRequestEditId(null)} />
          <p className="muted">Одна команда имеет один открытый запрос. Для следующей консультации закройте прежний с согласованным итогом и примите новый номер с новым временем готовности.</p>
        </Panel>

        <Panel title="Операционная очередь">
          <div className="actions"><Button type="button" variant="secondary" onClick={() => setShowClosed(!showClosed)}>{showClosed ? "Показать открытые" : "Показать также завершённые"}</Button></div>
          {requests.length === 0 && <p>В этой части очереди нет запросов. Примите вопрос команды через форму выше.</p>}
          <div className="stack">{requests.map((request) => <RequestCard key={request.id} request={request} data={data} analysis={analysis} disabled={blocked} edit={() => setRequestEditId(request.id)} patch={patch} />)}</div>
        </Panel>
      </>}
    </AppShell>
  );
}
