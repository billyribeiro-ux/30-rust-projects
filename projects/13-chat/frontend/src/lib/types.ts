export type User = {
  id: string;
  email: string;
  name: string;
  email_verified: boolean;
};

export type Room = {
  id: string;
  slug: string;
  name: string;
  created_by: string;
  created_at: string;
};

export type Message = {
  id: string;
  room_id: string;
  user_id: string;
  author: string;
  body: string;
  created_at: string;
};

/** Server-sent events from the WebSocket. Tagged union; matches the Rust
 *  `RoomEvent` enum exactly (with snake_case tag values). */
export type RoomEvent =
  | (Message & { type: 'message' })
  | { type: 'joined'; user_id: string; author: string }
  | { type: 'left'; user_id: string; author: string };

/** Client-sent frames over the WebSocket. */
export type ClientMsg = { type: 'send'; body: string };

export type FieldError = { field: string; message: string };
export type ApiError = {
  error: { code: string; message: string; fields: FieldError[] | null };
};
