# frozen_string_literal: true

class QueryRun < ApplicationRecord
  belongs_to :dataset, optional: true
  belongs_to :project, optional: true

  validates :sql, presence: true
  validates :status, inclusion: { in: %w[pending running succeeded failed] }
end
