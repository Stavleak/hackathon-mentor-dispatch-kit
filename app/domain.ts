export type RequestStatus = "queued" | "assigned" | "in_session" | "closed" | "cancelled";

export type Mentor = {
  id: string;
  name: string;
  skills: string[];
  languages: string[];
  available_from: string;
  available_until: string;
  enabled: boolean;
};

export type Request = {
  id: string;
  team_code: string;
  skill: string;
  language: string;
  question: string;
  queued_at: string;
  owner: string;
  deadline: string;
  next_step: string;
  status: RequestStatus;
  mentor_id: string | null;
  assigned_at: string | null;
  started_at: string | null;
  ended_at: string | null;
  closure_reason: string;
  manual_exception: string;
};

export type Data = {
  session_minutes: number;
  mentors: Mentor[];
  requests: Request[];
};

export type Analysis = {
  generated_at: string;
  open_count: number;
  overdue_ids: string[];
  recommendations: { request_id: string; mentor_id: string }[];
  unroutable_ids: string[];
  waiting_ids: string[];
  warnings: string[];
};

// Вымышленные учебные записи. Workbench сдвигает времена при явной загрузке демо.
export const demoData: Data = {
  session_minutes: 15,
  mentors: [
    {
      id: "DEMO-M-001", name: "Учебный технический ментор", skills: ["tech"], languages: ["ru", "kk"],
      available_from: "2026-10-10T10:00:00+05:00", available_until: "2026-10-10T12:00:00+05:00", enabled: true,
    },
    {
      id: "DEMO-M-002", name: "Учебный продуктовый ментор", skills: ["product", "pitch"], languages: ["ru"],
      available_from: "2026-10-10T10:00:00+05:00", available_until: "2026-10-10T12:00:00+05:00", enabled: true,
    },
  ],
  requests: [
    {
      id: "DEMO-Q-001", team_code: "DEMO-T-01", skill: "tech", language: "ru", question: "Как безопасно воспроизвести учебную ошибку API?",
      queued_at: "2026-10-10T10:01:00+05:00", owner: "DEMO-DISPATCH", deadline: "2026-10-10T10:06:00+05:00",
      next_step: "Назначить подходящего технического ментора", status: "queued", mentor_id: null, assigned_at: null,
      started_at: null, ended_at: null, closure_reason: "", manual_exception: "",
    },
    {
      id: "DEMO-Q-002", team_code: "DEMO-T-02", skill: "tech", language: "ru", question: "Как проверить формат учебного ответа?",
      queued_at: "2026-10-10T10:02:00+05:00", owner: "DEMO-DISPATCH", deadline: "2026-10-10T10:07:00+05:00",
      next_step: "Сообщить время следующего обновления", status: "queued", mentor_id: null, assigned_at: null,
      started_at: null, ended_at: null, closure_reason: "", manual_exception: "",
    },
    {
      id: "DEMO-Q-003", team_code: "DEMO-T-03", skill: "hardware", language: "kk", question: "Как проверить вымышленное подключение датчика?",
      queued_at: "2026-10-10T10:03:00+05:00", owner: "DEMO-DISPATCH", deadline: "2026-10-10T10:08:00+05:00",
      next_step: "Проверить резервного специалиста и сообщить честный вариант", status: "queued", mentor_id: null,
      assigned_at: null, started_at: null, ended_at: null, closure_reason: "", manual_exception: "",
    },
  ],
};
