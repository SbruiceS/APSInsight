# frozen_string_literal: true

class Plot < ApplicationRecord
  belongs_to :field
  has_many :soil_samples, dependent: :nullify

  validates :name, presence: true
end
