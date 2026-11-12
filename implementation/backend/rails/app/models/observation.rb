# frozen_string_literal: true

class Observation < ApplicationRecord
  belongs_to :soil_sample, optional: true
  belongs_to :dataset, optional: true

  validates :variable, presence: true
  validates :value, presence: true
  validates :unit, presence: true
  validates :observed_at, presence: true
end
