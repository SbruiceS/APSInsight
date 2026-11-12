# frozen_string_literal: true

class Membership < ApplicationRecord
  belongs_to :user
  belongs_to :organization
  belongs_to :project, optional: true
  belongs_to :role

  validates :user_id, uniqueness: { scope: [:organization_id, :project_id] }
end
