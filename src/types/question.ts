// 题目数据结构，与 Rust models/question.rs 保持一致

export interface DialogueTurn {
  speaker: "M" | "W";
  text: string;
}

export interface MultipleChoiceQuestion {
  id: number;
  question: string;
  options: Record<"A" | "B" | "C", string>;
  answer: "A" | "B" | "C";
}

export interface ShortDialogue {
  id: number;
  dialogue: DialogueTurn[];
  question: MultipleChoiceQuestion;
}

export interface LongDialogue {
  id: number;
  dialogue: DialogueTurn[];
  questions: [MultipleChoiceQuestion, MultipleChoiceQuestion];
}

export interface Monologue {
  text: string;
  questions: [MultipleChoiceQuestion, MultipleChoiceQuestion];
}

export interface TableRow {
  overview: string;
  details: string[]; // 含 `___NN___` 占位符
}

export interface SummaryTable {
  rows: [TableRow, TableRow, TableRow];
}

export interface RetellMaterial {
  passage: string;
  table: SummaryTable;
  blanks: Record<"15" | "16" | "17" | "18", string>;
}

export interface TestSession {
  session_id: string;
  short_dialogues: ShortDialogue[];
  long_dialogues: LongDialogue[];
  monologue: Monologue;
  retell: RetellMaterial;
  audio_paths: Record<string, string>;
}
