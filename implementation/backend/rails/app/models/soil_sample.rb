# frozen_string_literal: true

class SoilSample < ApplicationRecord
  belongs_to :field, optional: true
  belongs_to :plot, optional: true
  belongs_to :dataset, optional: true
  has_many :observations, dependent: :destroy

  validates :sampled_at, presence: true
  validates :depth_cm, numericality: { greater_than_or_equal_to: 0 }, allow_nil: true
end
