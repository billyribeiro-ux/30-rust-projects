export type Todo = {
  id: string;
  title: string;
  done: boolean;
  created_at: string;
  updated_at: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
  };
};
