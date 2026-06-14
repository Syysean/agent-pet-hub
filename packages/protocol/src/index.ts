/**
 * Unified Agent Event Protocol — Public API
 *
 * Re-exports all types, schemas, validators, and mappings
 * from the protocol package.
 *
 * @packageDocumentation
 */

// ─── Type Definitions ───────────────────────────────────────────────────────

export type {
  // Agent Source
  AgentSource,
  AgentIdentity,

  // Event Classification
  EventCategory,
  EventType,
  PetState,
  ErrorCode,

  // Core Event
  UnifiedAgentEvent,

  // WebSocket Protocol
  WSMessage,
  WSMessageType,
  WSMessagePayload,
  WSEventPayload,
  WSSubscribePayload,
  WSSubscribedPayload,
  WSErrorPayload,
  WSErrorCode,
  WSAuthPayload,
  WSAuthAckPayload,
  WSCommandPayload,
  WSCommandResultPayload,

  // TTS Protocol
  TTSEvent,
  TTSState,
  TTSSpeechRules,

  // Plugin Protocol
  PluginEvent,
  PluginAPI,
} from "./events.js";

// ─── Zod Schemas ────────────────────────────────────────────────────────────

export {
  // Primitive schemas
  AgentSourceSchema,
  EventCategorySchema,
  EventTypeSchema,
  PetStateSchema,
  ErrorCodeSchema,

  // Composite schemas
  UnifiedAgentEventSchema,
  WSMessageSchema,
  TTSEventSchema,
} from "./schemas.js";

// ─── Validators ─────────────────────────────────────────────────────────────

export {
  // Result type and helpers
  type Result,
  ok,
  err,

  // Validation functions
  validateEvent,
  tryValidateEvent,
} from "./validators.js";

// ─── Event Mappings ─────────────────────────────────────────────────────────

export {
  // Core types
  type EventMappingEntry,
  type EventMapping,

  // Per-agent mappings
  piEventMapping,
  hermesEventMapping,
  openclawEventMapping,

  // Registry
  allEventMappings,
  getEventMapping,
  findMappingEntry,
  getAllMappingEntries,
} from "./mapping.js";
