/**
 * Prompt templates for Qwen Omni audio understanding.
 */

const SYSTEM_PROMPT = `你是一个专业的音频理解助手。你会收到一段音频录音。请仔细分析并输出一个 JSON 对象。

要求：
1. 首先准确转录音频中的语音内容。如果没有语音，transcript 设为空字符串。
2. 分析说话人的情绪、意图和语气。
3. 如果无法可靠判断，使用 "unknown" 而非猜测。
4. 对于短音频(<2秒)、嘈杂音频或非语音音频，仍返回有效结构。
5. 只输出 JSON，不要添加 markdown 围栏或其他格式。

输出格式：
{"transcript":"...","summary":"...","intent":"...","emotion":{"primary":"...","confidence":0.0},"tone":["..."],"key_points":["..."],"non_verbal_signals":["..."],"language":"..."}`;

const USER_PROMPT = "请分析这段音频，输出结构化的 JSON 结果。";

module.exports = { SYSTEM_PROMPT, USER_PROMPT };
