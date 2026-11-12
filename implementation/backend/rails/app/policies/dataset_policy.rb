# frozen_string_literal: true

class DatasetPolicy < ApplicationPolicy
  def show?
    # Organization-scoped check would go here once auth is wired
    true
  end

  def create?
    true
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope.all # Later: scope.where(organization_id: user.organization_ids)
    end
  end
end
