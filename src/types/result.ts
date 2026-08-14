// 测试结果数据结构，与 Rust models/result.rs 保持一致

export interface McqResult {
  question_id: number;
  user_answer: "A" | "B" | "C" | null;
  correct_answer: "A" | "B" | "C";
  is_correct: boolean;
}

export interface BlankResult {
  blank_id: "15" | "16" | "17" | "18";
  user_answer: string;
  correct_answer: string;
  is_correct: boolean;
  score: number;
}

export interface RetellResult {
  score: number;
  max_score: number;
  comment: string;
  stt_text: string;
}

export interface TestResult {
  session_id: string;
  mcq_results: McqResult[];
  blank_results: BlankResult[];
  retell_result: RetellResult | null;
  blank_total_score: number;
  total_score: number;
  max_score: number;
  dialogue_texts: Record<string, string>;
}
