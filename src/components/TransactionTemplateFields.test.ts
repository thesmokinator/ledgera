import type { FormInstance } from "antd";
import { describe, expect, it } from "vitest";
import { transactionTemplateValidationFields } from "./TransactionTemplateFields";

function formWithPostings(count: number): FormInstance {
  return {
    getFieldValue: (name: string) => (name === "postings" ? Array.from({ length: count }) : undefined),
  } as unknown as FormInstance;
}

describe("transactionTemplateValidationFields", () => {
  it("includes date and movement fields for new transactions", () => {
    expect(
      transactionTemplateValidationFields({
        form: formWithPostings(2),
        transactionType: "movement",
        isEditing: false,
        includeDate: true,
      }),
    ).toEqual([
      "date",
      "code",
      "description",
      ["postings", 0, "account"],
      ["postings", 0, "amount"],
      ["postings", 0, "commodity"],
      ["postings", 0, "comment"],
      ["postings", 1, "account"],
    ]);
  });

  it("omits date and validates advanced posting rows for recurring templates", () => {
    expect(
      transactionTemplateValidationFields({
        form: formWithPostings(2),
        transactionType: "advanced",
        isEditing: true,
        includeDate: false,
      }),
    ).toEqual([
      "code",
      "description",
      "postings",
      ["postings", 0, "account"],
      ["postings", 0, "commodity"],
      ["postings", 0, "amount"],
      ["postings", 0, "unitPrice"],
      ["postings", 0, "comment"],
      ["postings", 1, "account"],
      ["postings", 1, "commodity"],
      ["postings", 1, "amount"],
      ["postings", 1, "unitPrice"],
      ["postings", 1, "comment"],
    ]);
  });
});
